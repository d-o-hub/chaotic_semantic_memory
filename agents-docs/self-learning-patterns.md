# Self-Learning Patterns

Key patterns curated from iterations. For full history, see `@progress/LEARNINGS.md`.

## What Works
1. Systematic codebase analysis before planning
2. ADRs for every non-trivial architectural change
3. Domain-specific debugging skills over generic boilerplate
4. Executable scripts in skills — agents can run them directly
5. Seeded RNG (`StdRng::seed_from_u64(42)`) for deterministic tests
6. CI-enforced version synchronization

## Technical Insights
- Dense `Array2<f32>` for 50k×50k reservoir is infeasible (~10 GB). CSR with k=64 → ~25 MB.
- `HVec10240::permute()` with `bit_shift == 0` causes UB — must guard
- `Arc<RwLock<Connection>>` for libsql is unsafe under tokio. Per-op `connect()` is cheap.
- Always use `f32::total_cmp()` for similarity sorting — `partial_cmp().unwrap()` panics on NaN
- `inject_text()` does NOT store text — use `inject_text_with_metadata()` with `("_text", text)`
- Min-max normalization amplifies noise — low HDC scores (~0.12) become 1.0

## What to Avoid
- Dense matrices for reservoirs > ~2000 nodes
- Sharing libsql `Connection` across async tasks via RwLock
- `partial_cmp().unwrap()` on floats
- `Vec<(String, f32)>` for associations — use `HashMap<String, f32>`
- Multiple scripts with overlapping functionality — merge them
- Archived GitHub repos as dependencies — fork or find active alternative

## Learning Loop
See `@progress/LEARNINGS.md` for full history. After each iteration:
1. Record non-obvious discoveries
2. Update module LOC counts
3. Run test + bench gates
