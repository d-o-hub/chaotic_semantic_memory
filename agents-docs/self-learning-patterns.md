# Self-Learning Patterns

Key patterns recorded from iterations (see @progress/LEARNINGS.md for full history).

## What Works

1. Systematic codebase analysis before planning — found more real issues than GOAP state listed
2. Using oracle for deep code review across all modules simultaneously
3. Writing ADRs for every non-trivial architectural change before implementation
4. Creating domain-specific debugging skills rather than generic boilerplate
5. Adding executable scripts to skills — agent can run them directly
6. Treating GOAP state booleans as executable acceptance criteria
7. Using seeded RNG (`StdRng::seed_from_u64(42)`) in tests for determinism
8. Migrating to `libsql::Builder` to remove deprecated API usage
9. Enabling `PRAGMA foreign_keys = ON` per-connection for deterministic FK behavior
10. CI-enforced version synchronization — catches drift before merge

## Technical Insights

- Dense `Array2<f32>` for 50k×50k reservoir is infeasible (~10 GB). CSR with k=64 reduces to ~25 MB.
- `HVec10240::permute()` with `bit_shift == 0` causes undefined behavior — must guard
- `Arc<RwLock<Connection>>` for libsql is unsafe under tokio. Per-operation `connect()` is cheap and eliminates Send/Sync risks
- Always use `f32::total_cmp()` for similarity sorting — `partial_cmp().unwrap()` panics on NaN
- `Vec<Vec<(usize, f32)>>` incurs substantial allocator overhead; contiguous CSR buffers are faster
- For large sparse reservoirs, memory locality can dominate runtime more than arithmetic throughput
- **inject_text() does NOT store text content** — must use inject_text_with_metadata() with `("_text", text)` for retrieval
- **probe_text() uses pure HDC similarity** — for short queries (1-2 tokens), BM25 hybrid with 90% keyword weight is better
- **HDC returns low-similarity noise** — scores ~0.12 for unrelated documents. Must filter with threshold before hybrid merge
- **Min-max normalization amplifies noise** — low HDC scores (0.12) become 1.0, competing with correct BM25 results

## What to Avoid

- Do not use dense matrices for reservoirs > ~2000 nodes
- Do not share a single libsql `Connection` across async tasks via RwLock
- Do not use `partial_cmp().unwrap()` on floats
- Do not assume `Vec<(String, f32)>` associations deduplicate — use `HashMap<String, f32>`
- Do not use `cargo bench -- --baseline` (without `--bench benchmark`) — libtest benches interfere
- Do not suppress deprecated libsql constructors long-term — migrate to `Builder`
- Do not relax spectral-radius guardrails to chase speed
- Do not pool connections for local SQLite (no benefit, adds overhead)
- Do not make versioning mandatory (should be opt-in)
- Do not create multiple scripts with overlapping functionality — merge related scripts (e.g., version checking into link checking)
- Do not use archived GitHub repositories as dependencies — always find an active alternative or fork and maintain
- Do not hardcode version numbers in test fixtures or examples — use current version or verify sync
- **Do not use inject_text() when you need to retrieve the original text later** — use inject_text_with_metadata() instead

## Learning Loop

After each iteration:
1. Record what worked in @progress/LEARNINGS.md.
2. Record progress in @progress/PROGRESS.md.
3. Update module LOC counts.
4. Run test + bench gates.
5. Commit with Conventional Commits format (see `git-workflow` skill).
