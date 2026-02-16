# Commit Types and Scopes

## Types

| Type | Use When | Example |
|------|----------|---------|
| `feat` | New feature or capability | `feat(reservoir): add SIMD cosine similarity` |
| `fix` | Bug fix or correction | `fix(persistence): handle FK violations` |
| `perf` | Performance improvement | `perf(hyperdim): parallel bundle operations` |
| `refactor` | Code change, no behavior change | `refactor(singularity): extract search module` |
| `test` | Adding/fixing tests | `test(reservoir): add edge case coverage` |
| `docs` | Documentation changes | `docs(adr): add ADR-0013 for SIMD` |
| `chore` | Maintenance tasks | `chore(deps): update libsql to 0.5` |

## Scopes

| Scope | Module | Files |
|-------|--------|-------|
| `hyperdim` | Hypervector operations | `src/hyperdim.rs` |
| `reservoir` | Echo state network | `src/reservoir.rs` |
| `singularity` | Concept store | `src/singularity.rs` |
| `persistence` | libSQL storage | `src/persistence.rs` |
| `framework` | High-level API | `src/framework.rs` |
| `wasm` | WASM bindings | `src/wasm.rs` |
| `ci` | CI/CD pipeline | `.github/workflows/` |
| `skills` | Agent skills | `.agents/skills/` |
| `adr` | Architecture decisions | `plans/adr/` |
| `planning` | GOAP state/actions | `plans/` |

## Examples

### Simple feature
```
feat(reservoir): add SIMD-accelerated cosine similarity

Use std::simd::u128x2 for parallel word operations.
Improves batch similarity throughput by 3x.
```

### Breaking change
```
fix(persistence): enforce foreign key constraints

Enable PRAGMA foreign_keys = ON for all connections.

BREAKING CHANGE: existing databases without FK support may fail
```

### Multiple changes
```
perf(hyperdim,framework): optimize batch operations

- Use par_chunks for hypervector bundling
- Add inject_concepts() batch API to framework
- Reduce allocation overhead in hot paths
```

### Documentation
```
docs(adr): add ADR-0013 for SIMD hypervector operations

Documents decision to use std::simd for vectorized ops.
```

## Commit Message Checklist

- [ ] Type is one of: feat, fix, perf, refactor, test, docs, chore
- [ ] Scope matches affected module
- [ ] Summary is imperative ("add" not "added")
- [ ] Summary is ≤ 50 characters
- [ ] Body explains what and why, not how
- [ ] Breaking changes noted in footer
