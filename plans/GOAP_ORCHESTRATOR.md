# GOAP Orchestrator State

## Target State
- All workspace extraction issues resolved
- CI passes for all crates
- Documentation reflects workspace structure

## Action Plan

### Action 1: Issue #374
- **Issue**: #374
- **Title**: docs: regenerate `llms.txt`, `llms-full.txt`, and `export.json` to reflect workspace crate structure from PR #356
- **Labels**: none
- **Status**: queued
- **Branch**: feat/issue-374-docs-regenerate-llms-txt-llms-full-txt-and-export-

### Action 2: Issue #373
- **Issue**: #373
- **Title**: ci: update `.github/workflows/ci.yml` and `pre-release-gate.yml` for Cargo workspace with per-crate test and mutation jobs
- **Labels**: none
- **Status**: queued
- **Branch**: feat/issue-373-ci-update-github-workflows-ci-yml-and-pre-release-

### Action 3: Issue #372
- **Issue**: #372
- **Title**: ci: add WASM32 compilation check for `csm-core` to CI/CD pipeline
- **Labels**: none
- **Status**: queued
- **Branch**: feat/issue-372-ci-add-wasm32-compilation-check-for-csm-core-to-ci

### Action 4: Issue #371
- **Issue**: #371
- **Title**: refactor: remove bridge/stub modules from main crate `src/lib.rs` after full workspace extraction
- **Labels**: none
- **Status**: queued
- **Branch**: feat/issue-371-refactor-remove-bridge-stub-modules-from-main-crat

### Action 5: Issue #370
- **Issue**: #370
- **Title**: chore: finalize root `Cargo.toml` workspace members and shared dependency versions
- **Labels**: none
- **Status**: queued
- **Branch**: feat/issue-370-chore-finalize-root-cargo-toml-workspace-members-a

### Action 6: Issue #369
- **Issue**: #369
- **Title**: refactor: extract WASM bindings into a dedicated `csm-wasm` workspace crate
- **Labels**: none
- **Status**: queued
- **Branch**: feat/issue-369-refactor-extract-wasm-bindings-into-a-dedicated-cs

### Action 7: Issue #368
- **Issue**: #368
- **Title**: refactor: extract CLI commands into a standalone `csm-cli` workspace crate
- **Labels**: none
- **Status**: queued
- **Branch**: feat/issue-368-refactor-extract-cli-commands-into-a-standalone-cs

### Action 8: Issue #367
- **Issue**: #367
- **Title**: refactor: create `csm-persistence` workspace crate for libSQL/Turso storage backend
- **Labels**: none
- **Status**: queued
- **Branch**: feat/issue-367-refactor-create-csm-persistence-workspace-crate-fo

### Action 9: Issue #366
- **Issue**: #366
- **Title**: refactor: create `csm-retrieval` workspace crate for graph-RAG, reranking, and BM25 logic
- **Labels**: none
- **Status**: queued
- **Branch**: feat/issue-366-refactor-create-csm-retrieval-workspace-crate-for-

### Action 10: Issue #365
- **Issue**: #365
- **Title**: refactor: finalize `csm-memory` crate extraction and resolve circular dependency on persistence/retrieval
- **Labels**: none
- **Status**: queued
- **Branch**: feat/issue-365-refactor-finalize-csm-memory-crate-extraction-and-

### Action 11: Issue #364
- **Issue**: #364
- **Title**: refactor: complete extraction of `csm-embedding` into standalone workspace crate
- **Labels**: jules
- **Status**: queued
- **Branch**: feat/issue-364-refactor-complete-extraction-of-csm-embedding-into

### Action 12: Issue #353
- **Issue**: #353
- **Title**: Wave 24: Quantized Binary Hypervectors (ADR-0075)
- **Labels**: none
- **Status**: queued
- **Branch**: feat/issue-353-wave-24-quantized-binary-hypervectors-adr-0075-

