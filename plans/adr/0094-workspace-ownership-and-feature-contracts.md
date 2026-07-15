# ADR-0094: Workspace Ownership and Feature Contracts

## Status

Proposed (2026-07-14)

## Context and Problem Statement

The repository extracted reusable crates (`csm-core`, `csm-memory`, `csm-retrieval`, `csm-persistence`, `csm-cli`, `csm-wasm`, and others), but the root crate still contains duplicate implementations. The duplication is not uniformly generated or delegated:

- all root retrieval implementation files differ from `csm-retrieval` counterparts;
- most root/standalone CLI files are byte-identical copies, while some differ;
- root and `csm-wasm` binding files are duplicated and partly divergent;
- root and extracted persistence modules differ in generic/vector-format behavior;
- export payload types exist in parallel forms.

The feature surface also does not match its advertised boundaries. `cargo tree -p chaotic_semantic_memory --no-default-features -e features` still resolves `csm-persistence` default features, libSQL, and Rayon. When persistence is disabled, builder methods silently ignore database configuration and fallback persistence methods return false-success empty/`Ok` results.

Protocol and distribution boundaries have drifted too: MCP advertises 80 `u128` JSON integers but parses only `u64`, and CI builds `crates/csm-wasm` while release publishes a root build.

## Decision Drivers

- Each behavior must have one implementation owner.
- The root package must preserve stable public paths during migration.
- Optional features must be genuinely optional in dependency resolution and behavior.
- Disabled capabilities must fail explicitly rather than silently succeed.
- CI must test the same artifact that release publishes.
- The full 10,240-bit hypervector contract must survive protocol round trips.

## Considered Options

1. Continue dual maintenance with periodic synchronization.
2. Collapse back to a monolith.
3. Make workspace crates canonical and the root a façade/orchestrator.
4. Generate duplicate root sources from canonical crate sources.

## Decision Outcome

Chosen option: **workspace crates own implementations; the root crate is a compatibility façade and orchestration layer; migration is incremental by concern**.

### Ownership rules

- `csm-core`: hypervectors, encoders, reservoirs, low-level kernels.
- `csm-memory`: concepts, graph, metadata filters, ANN indexes, retrieval state.
- `csm-retrieval`: BM25, hybrid scoring, GraphRAG, reranking algorithms and shared result contracts.
- `csm-persistence`: durable schema and CRUD over owner-neutral trait types.
- `csm-cli`: argument/command implementation and binary entry behavior.
- `csm-wasm`: canonical JS/WASM bindings and npm build target.
- root `chaotic_semantic_memory`: framework orchestration, compatibility re-exports, feature composition, and root-specific adapters only.

An ownership map must identify every public module and type. A root file may re-export, delegate, or adapt; it may not retain a second algorithm implementation.

### Migration order

1. Retrieval and shared result/abstention types.
2. Persistence and export payload contracts.
3. CLI and WASM surfaces.

Each step preserves root public paths for at least one compatibility window and adds API/behavior parity tests before duplicate removal. Blind re-export is prohibited where behavior currently differs.

### Feature contract

- Root owner dependencies use `default-features = false` where the root controls feature composition.
- Root `persistence`, `parallel`, ANN, embedding, and protocol features explicitly forward to owning crates.
- `--no-default-features` excludes libSQL, persistence runtime, CLI, and Rayon unless another selected feature requires them.
- Feature-disabled APIs are cfg-absent or return `UnsupportedOperation`; builder configuration is never silently discarded.
- All workspace manifests use the canonical workspace MSRV.

### Protocol and artifact contract

- MCP uses a lossless JSON-safe representation: canonical base64 bytes is preferred; two `u64` halves per word is acceptable only if documented and tested. Raw JSON `u64`/`u128` numbers are not the default wire format.
- `csm-wasm` becomes the canonical npm artifact. CI, freshness checks, size checks, JS smoke tests, and release run the same command and package path.

## Positive Consequences

- Fixes land once and reach every distribution channel.
- Optional dependency promises become testable.
- Root API compatibility can be preserved while internal ownership improves.
- WASM CI validates what npm users receive.
- Protocol schemas describe encodings clients can safely produce.

## Negative Consequences

- Migration requires temporary adapters and parity tests.
- Some root-specific types must move or be generalized before re-export is possible.
- Individual crate APIs may expand to support the façade.
- The standalone crates and root package must coordinate versions during the compatibility window.

## Pros and Cons of the Options

### Dual maintenance

- Good, because no migration is required.
- Bad, because verified divergence and duplicate tests continue to grow.

### Monolith

- Good, because ownership is obvious.
- Bad, because it discards the intended reusable crate ecosystem and already-completed extraction work.

### Workspace owners plus façade

- Good, because it combines reuse with compatibility.
- Good, because features can be forwarded explicitly.
- Bad, because careful staged migration is required.

### Generated duplicate sources

- Good, because drift can be mechanically prevented.
- Bad, because generated Rust obscures ownership and still duplicates compiled/tested surfaces.

## TRIZ Rationale

- **Taking out:** remove algorithm bodies from the façade.
- **Intermediary:** compatibility adapters preserve public paths while owners change.
- **Segmentation:** migrate retrieval, persistence, and distribution surfaces independently.

## Follow-up Actions

- `enforce_workspace_feature_contracts`
- `replace_persistence_disabled_noops`
- `fix_mcp_hypervector_wire_format`
- `align_wasm_ci_release_artifact`
- `consolidate_retrieval_ownership`
- `consolidate_persistence_cli_wasm_ownership`
- `complete_workspace_ci_and_supply_chain_matrix`
- `deduplicate_test_and_source_surfaces`

## Acceptance Criteria

- An ownership manifest covers every public root/workspace module.
- No independently maintained duplicate algorithm body remains after its migration phase.
- Root default, root no-default, each ANN feature, standalone CLI, standalone persistence, and canonical WASM build pass.
- `cargo tree --no-default-features` contains no libSQL or Rayon unless explicitly requested.
- Persistence-disabled configuration cannot return false success.
- MCP round trips vectors with high bits set in every `u128` word.
- CI and release produce and smoke-test the same WASM package.
- Workspace CI inventory includes every package and is checked against `cargo metadata`.
