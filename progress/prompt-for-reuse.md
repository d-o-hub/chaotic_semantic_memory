## Research & Implement: Academic Paper Review for chaotic_semantic_memory
## Run date: [INSERT_DATE]

### Step 0 — Determine Search Window

1. Read `.jules/paper-research-last-run.txt`
   - If the file exists: use its date as `LAST_RUN_DATE`
   - If the file does not exist: use `2026-01-01` as `LAST_RUN_DATE` (cold start)
2. Set `THIS_RUN_DATE = [INSERT_DATE]`
3. If `LAST_RUN_DATE >= THIS_RUN_DATE`: **stop immediately** — this run has already been executed. Do nothing.
4. Proceed only if `LAST_RUN_DATE < THIS_RUN_DATE`

---

### Step 1 — Academic Research (papers published after LAST_RUN_DATE)

Search the web for papers published **strictly after `LAST_RUN_DATE`** on topics relevant to this codebase. You may use a script connecting to the arXiv API.

1. **Chaotic maps for semantic hashing / vector indexing**
2. **Echo State Networks (ESN) / Reservoir Computing**
3. **Approximate Nearest Neighbor (ANN) search improvements**
4. **Locality-Sensitive Hashing (LSH) with chaotic or non-linear projections**
5. **Semantic memory consolidation**
6. **Forgetting curves and adaptive decay**
7. **Quantization-aware similarity search**
8. **Self-organizing maps (SOM) / competitive learning**
9. **Topological data analysis (TDA) / persistent homology**
10. **In-context learning memory augmentation**

For each relevant paper found, document in `progress/paper-research-[INSERT_DATE].md`:
- Title, authors, publication date, arXiv/DOI link
- Core claim / technique
- Potential integration point (`src/`, `benches/`, `wasm/`, CLI)
- Estimated impact: HIGH / MEDIUM / LOW

If no papers newer than `LAST_RUN_DATE` are found, write that to the progress file and jump directly to Step 6.

---

### Step 2 — Codebase Mapping

Read to understand current architecture:
- `README.md`, `AGENTS.md`, `CLAUDE.md`, `Cargo.toml`
- `src/` (all files)
- `benches/` and `benchmarks/`
- `tests/`
- `cli-npm/`
- `.jules/`

---

### Step 3 — Implementation (HIGH impact only)

For each HIGH-impact paper newer than `LAST_RUN_DATE`:

1. Implement as a non-breaking addition — feature flags, new struct variants, or new modules. Never remove or break existing behavior.
2. Follow existing Rust code style: idiomatic Rust, `no_std`-compatible where relevant, `#[inline]`, proper error handling with `thiserror`/`anyhow` as already used.
3. Add or extend:
   - **Unit tests** in `tests/` or inline `#[cfg(test)]`
   - **Criterion benchmarks** in `benches/`
   - **Evaluation output** in `benchmarks/`
   - **Dog food validation**: run through CLI (`cli-npm/`) on real inputs

---

### Step 4 — Validation Gate

Only proceed to PR if ALL pass:

- [ ] `cargo test --all` — zero regressions
- [ ] `cargo bench` — measurable improvement or no regression on existing benchmarks
- [ ] New benchmark demonstrates the paper's claimed improvement
- [ ] Database / file output is structurally valid
- [ ] No `unsafe` added without a `// SAFETY:` comment

If gate fails → write findings to `progress/paper-research-[INSERT_DATE].md` only. No PR.

---

### Step 5 — PR (if gate passes)

Create a git branch (`research/[short-paper-slug]-[YYYY-MM]`), commit, and submit using the standard tool.

---

### Step 6 — Update Last Run Date (ALWAYS)

Write `THIS_RUN_DATE` to `.jules/paper-research-last-run.txt`
