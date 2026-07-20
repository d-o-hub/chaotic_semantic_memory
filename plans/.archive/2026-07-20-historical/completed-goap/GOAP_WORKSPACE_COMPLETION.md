# GOAP Plan: Workspace Extraction Completion

## Target State
- All 12 GitHub issues resolved (#364-#374, #382)
- Clean workspace with no circular dependencies
- WASM builds pass CI
- All tests pass

## Current State (2026-06-12)
- ✅ #364: csm-embedding extracted
- ✅ #365: csm-memory extracted
- ✅ #366: csm-retrieval extracted
- ✅ #367: csm-persistence extracted
- ✅ #368: csm-cli extracted
- ❌ #382: csm-traits (BLOCKS #369, #371)
- ❌ #372: mio WASM fix (BLOCKS CI)
- ❌ #369: csm-wasm (BLOCKED by #382)
- ❌ #371: remove stubs (BLOCKED by #382)
- ❌ #370: finalize workspace
- ❌ #373: update CI/CD
- ❌ #374: regenerate docs

## Action Plan (Dependency Order)

### Phase 1: Unblock CI (P0)
- [x] Action 1: Fix mio WASM incompatibility (#372)
  - Pre: none
  - Effect: CI wasm/lint jobs pass
  - Cost: 1 (5 minutes)
  - PR: #383 merged

### Phase 2: Create Shared Types (P0)
- [x] Action 2: Create csm-traits crate (#382)
  - Pre: none
  - Effect: Shared types available for all crates
  - Cost: 8 (2-3 hours)
  - PR: #384 merged

### Phase 3: Extract WASM (P1)
- [x] Action 3: Extract csm-wasm (#369)
  - Pre: #382 complete
  - Effect: WASM bindings in standalone crate
  - Cost: 4 (1-2 hours)
  - PR: #385 merged

### Phase 4: Cleanup (P2)
- [ ] Action 4: Remove bridge/stub modules (#371)
  - Pre: #382 complete
  - Effect: No duplicate source files
  - Cost: 2 (30 minutes)

- [ ] Action 5: Finalize workspace members (#370)
  - Pre: #382, #369 complete
  - Effect: Workspace fully configured
  - Cost: 1 (15 minutes)

### Phase 5: CI & Docs (P3)
- [ ] Action 6: Update CI/CD (#373)
  - Pre: #372, #382 complete
  - Effect: Per-crate testing in CI
  - Cost: 4 (1 hour)

- [ ] Action 7: Regenerate docs (#374)
  - Pre: #370, #382 complete
  - Effect: Documentation reflects workspace
  - Cost: 2 (30 minutes)

## Execution Summary

| Action | Issue | PR | Status |
|--------|-------|----|--------|
| Fix mio WASM | #372 | - | queued |
| Create csm-traits | #382 | - | queued |
| Extract csm-wasm | #369 | - | queued |
| Remove stubs | #371 | - | queued |
| Finalize workspace | #370 | - | queued |
| Update CI/CD | #373 | - | queued |
| Regenerate docs | #374 | - | queued |
