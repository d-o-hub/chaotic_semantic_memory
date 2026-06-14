# Issue Dependency Graph

## Open Issues (12)

### Workspace Extraction (Ordered by Dependency)

```
#364: csm-embedding extraction (has jules label)
  └─→ #365: csm-memory extraction (depends on csm-core)
       └─→ #366: csm-retrieval extraction
            └─→ #367: csm-persistence extraction
                 └─→ #368: csm-cli extraction
                      └─→ #369: csm-wasm extraction
                           └─→ #370: finalize workspace members
                                └─→ #371: remove bridge/stub modules
                                     └─→ #373: update CI/CD
                                          └─→ #374: regenerate docs

#372: WASM32 compilation check (independent, can run in parallel)
```

## Dependency Matrix

| Issue | Depends On | Blocks | Priority |
|-------|------------|--------|----------|
| #364 | csm-core (done) | #365 | HIGH |
| #365 | #364 | #366 | HIGH |
| #366 | #365 | #367 | HIGH |
| #367 | #366 | #368 | HIGH |
| #368 | #367 | #369 | MEDIUM |
| #369 | #368 | #370 | MEDIUM |
| #370 | #369 | #371 | MEDIUM |
| #371 | #370 | #373 | LOW |
| #372 | none | #373 | LOW |
| #373 | #371, #372 | #374 | MEDIUM |
| #374 | #373 | none | LOW |

## Parallel Execution Opportunities

- #372 (WASM32 check) can run alongside #364-#369
- #364-#369 are sequential (dependency chain)
- #370-#374 are sequential (finalization)
