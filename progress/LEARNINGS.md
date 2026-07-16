# LEARNINGS - Chaotic Semantic Memory

## Core Patterns & Best Practices

### Security Patterns
- **Path Hijacking Mitigation (CWE-426)**: Always resolve system executables (e.g., `git`) to absolute paths. Filter the `PATH` environment variable to strictly exclude relative entries like `.` or empty paths before use in `Command::new()`.
- **DoS Prevention**: Enforce strict upper bounds on all public API parameters. For graph traversals, use `MAX_TRAVERSAL_DEPTH = 32` and `MAX_TRAVERSAL_RESULTS = 10,000`. Batch operations should be limited to `max_batch_size` (default 1000).
- **Namespace Input Validation (CWE-770)**: Enforce strict length (128 bytes), non-empty, and control-character filtering on namespaces across all public APIs to prevent resource exhaustion and undefined isolation behavior.

### Optimization Patterns
- **Instruction-Level Parallelism (ILP)**: Use independent accumulators (e.g., 4) in hot loops (popcount, dot products) to break serial dependency chains. This often outperforms SIMD due to avoiding STLF stalls.
- **Branchless Logic**: Use branchless bitmask construction (`w |= (condition as u128) << j`) to minimize branch misprediction penalties in tight loops.
- **Zero-Allocation Interning**: Use `Arc<str>` interning for terms (e.g., BM25) to share memory. Use a `get_mut`/`get_key_value` double-lookup pattern to update frequencies without new allocations for existing terms.
- **Algorithmic Parallelism**: Parallelize $O(N)$ scans using Rayon's `par_iter()`. For bitmask-based retrieval, this reduces latency from $O(N)$ to $O(N/P)$.
- **Bitmask Filtering**: For power-of-2 bucket counts, replace modulo (`%`) with bitmask AND (`&`) for significant speedups.

### Performance Baselines (x86_64)
- `HVec10240::hamming_distance`: ~219 ns (unrolled GPR loop).
- `HVec10240::cosine_similarity`: ~238 ns (unified GPR path).
- Reservoir Step (50k nodes): ~136 µs (4 accumulators).
- BM25 Search (10k docs): ~406 µs (hoisted constants).

## Technical Insights by Module

### Reservoir & ESN
- **Partial Updates**: Partitioned updates (rotating node subsets) must preserve momentum. Reverting to a partial `prev_state` update loop is critical for maintaining correct inertial state in partitioned ESNs.
- **Memory Locality**: For large sparse reservoirs, CSR-like contiguous arrays and neighborhood connectivity dominate throughput by reducing random memory access.

### Retrieval & Similarity
- **Unified Similarity**: Deriving Cosine Similarity from Hamming Distance (`1.0 - (dist / 5120.0)`) for bipolar hypervectors is optimal across all architectures, saving a `NOT` instruction and unifying SIMD/scalar paths.
- **Top-K Selection**: Use `select_nth_unstable_by` for $O(N)$ partial sorting instead of $O(N \log N)$ full sort.

### Persistence & Environment
- **Namespace Isolation**: Use prefixed table names (`csm_concepts`, etc.) for shared databases. Legacy names in migration scripts are intentional.
- **WASM Compliance**: Always gate `rayon` and I/O paths with `#[cfg(not(target_arch = "wasm32"))]`. Use `wasm-bindgen-futures` for async WASM exports.
- **Sandbox Limits**: Intermittent network timeouts during `cargo fetch` are common. Verify core logic using standalone `rustc` scripts if dependencies are minimal.

## What to Avoid
- **Unqualified Commands**: Never use bare command names like `Command::new("git")`.
- **Insecure PATH**: Never trust `PATH` to contain only absolute, trusted directories.
- **Floating Point Comparison**: Never use `partial_cmp().unwrap()` on floats; NaN will panic. Use `f32::total_cmp()`.
- **Schema Drift**: Always update all persistence surfaces (single, batch, export, WASM) when adding concept fields.
- **Dense Matrices**: Avoid dense matrices for reservoirs > 2000 nodes; use CSR.
- **Archived Deps**: Never use archived GitHub repositories as dependencies.

## Autonomous PR Repair (Root Cause Analysis - May 2026)
- **Versioning Logic**: Native framework versioning (v1, v2) is triggered by updating a concept with a stable ID. Manually appending suffixes like `:v1` in the benchmark generator creates separate concepts and breaks lineage evaluation.
- **Merge Conflict Strategy**: When rebasing or merging, prioritize preserving functional logic from both sides. In this task, Association/Isolation tasks from main were successfully integrated with the expanded coverage areas (TTL, Bridge, History).
- **Tool Discipline**: Use `git checkout origin/main -- <file>` to restore files lost during complex merges or rebases.
- **Optimization Strategy**: Gating parallelization (e.g., Rayon) with a minimum workload threshold (N >= 32) prevents task scheduling overhead from dominating small operations, yielding order-of-magnitude gains in hot paths like hypervector bundling.

## State Drift Verification (Wave 21 P0 — May 2026)
- **Built ≠ Installed**: `~/.local/bin/csm` (or any global install) frequently lags source by multiple releases. Before claiming a CLI surface is missing a command, build locally and check `./target/debug/csm --help`. The Wave 21 P0 gap analysis falsely reported missing CLI subcommands until this was verified — they were already wired in source since Wave 20.
- **GOAP_STATE drift**: `plans/GOAP_STATE.md` is a flat YAML mapping. Duplicate keys (e.g. `action_last_completed` appearing 3×) are silently overwritten by the last one. Always `grep -c '^  action_last_completed' plans/GOAP_STATE.md` → must equal 1.
- **ACTIONS.md drift**: Newly merged ADRs (e.g. DuckDB 0079-0082) can ship to `main` without corresponding rows in `plans/ACTIONS.md`. Backfill complete rows when discovered so the planner has accurate state.
- **Registry ↔ Disk parity**: Added `scripts/check-adr-parity.sh` to enforce `plans/ADR_REGISTRY.md` ↔ `plans/adr/` + `docs/adr/`. Allow rows marked `Superseded` / `N/A`; warn on orphan files; error on missing-with-backing.
- **Jules delegation pattern**: Long actions (`cost ≥ 12` in ACTIONS.md) belong in a GitHub issue labeled `jules` rather than an interactive session. Mark the action `status: delegated` with `jules_issue: <num>` so the planner sees the dependency satisfied via remote execution. Wave 21 MCP server (ADR-0067) was delegated this way to issue #246.

## Memory Lifecycle Verification (2026-05-18)

- **sqld HTTP API for local DB verification**: The `sqld` binary from Turso (`~/.turso/sqld`) can serve local `.db` files and expose them via an HTTP API at `/v1/query`. However, it binds to the grpc port (50051) by default and may fail with "File exists" on stale sockets. Use `--http-listen-addr 127.0.0.1:<port>` and ensure no stale sqld processes remain (`pkill -9 sqld`). For simple SELECT queries, Python's `sqlite3` module works on libSQL databases (table names are prefixed with `csm_`).
- **CLI table name prefix**: The persistence layer uses `csm_`-prefixed table names (`csm_concepts`, `csm_associations`, `csm_schema_version`). Direct SQL access must account for this prefix.
- **No native archive command**: The CLI has `delete` but no `archive`. Archive is handled via marker concepts with metadata `{"status":"archived","target":"<id>"}`. If archive becomes a common workflow, consider adding a native `csm archive <id>` command.
- **Export/import roundtrip fidelity**: JSON export preserves metadata, vector data, and associations. Import into a fresh DB produces identical probe results (same similarity scores), confirming no precision loss in serialization.

### CI/CD Maintenance
- **Node 20 Deprecation**: GitHub Actions using Node 20 runtime can be resolved by upgrading to versions that natively support Node 24 (e.g., `actions/checkout@v5`, `Swatinem/rust-cache@v2.9.1`).
- **Miri Job Reliability**: Miri tests are significantly slower than standard tests. For a suite of ~220 tests, a 30-minute timeout is often insufficient, and 60 minutes is a safer baseline for initial reliability.
- **Action Pinning**: Use `git ls-remote --tags <url>` to find the exact SHA for a specific version tag to ensure security and reproducible CI environments.

## 2026-06-05 — Namespace parameter missing bounds/character validation
**Vulnerability:** `set_namespace`, `delete_namespace`, `export_namespace`, `export_namespace_to_bytes`, and `FrameworkBuilder::with_namespace` accepted arbitrary strings with no length, emptiness, or control-character checks. The namespace is used as a DB primary key prefix in every libsql query.
**Learning:** The `validate_concept_id` pattern existed and was thorough, but was not applied to the analogous `namespace` input surface when those APIs were added. New public API parameters that become DB keys need the same treatment.
**Prevention:** When adding any parameter that becomes part of a DB key, hash map key, or file path: apply validate_concept_id-style guards (empty check, byte limit, control-char reject) before first use.
## Mutation Testing Discipline (2026-06-05, PR #346)
- **Unreachable code is a mutation smell**: When refactoring a `visited.contains(&id) || depth > max_depth` style guard into `!visited.insert(id.clone())`, audit the queue invariant. If new entries are only enqueued at `depth + 1` when `depth < max_depth`, the original `depth > max_depth` check becomes unreachable and cargo-mutants will catch the `||` -> `&&` substitution as a missed mutant. Remove the dead branch and document the invariant.
- **Mutation test cost is acceptable for PR fixes**: `cargo-mutants --in-diff <pr.diff> --in-place --no-shuffle` against a 35-line PR diff takes ~14 minutes locally and exercises both the original and mutated compile/test paths. 11 mutants → 10 caught, 1 unviable, 0 missed is a strong signal the fix is correct.
- **Always run `--in-diff` on the post-fix tree**: cargo-mutants reports the baseline, so the diff must reflect the fix (not the pre-fix PR head). Generate the diff with `git diff origin/main > /tmp/pr.diff` after the fix is staged.

## Workspace LOC Gate Enforcement (2026-07-11, Wave 31)

### Problem: LOC gate only checked `src/`, not workspace crates
- After workspace extraction (PRs #377-#385), code moved to `crates/*/src/` but the
  LOC gate (`scripts/validate.sh`) only scanned `find src -name '*.rs'`.
- Jules bot PRs added code to workspace crates without any enforcement, growing 3 files
  past 500 LOC undetected (singularity.rs 629, hyperdim.rs 563, graph_traversal.rs 517).

### Fix
- Extended `scripts/validate.sh` to scan `find src crates -name '*.rs' -not -path '*/target/*'`
- Updated `AGENTS.md` and `agents-docs/hard-constraints.md` to reflect workspace coverage
- Split the 3 violating files using established patterns (types extraction, trait extraction, test extraction)

### Prevention
- The LOC gate now covers all first-party Rust code automatically
- When creating new workspace crates, verify they are within the `crates/` directory
  which is already covered by the LOC gate scan

### Commitlint Scope Maintenance
- **Always add new scopes when creating workspace crates or packages**: When adding
  a new crate (e.g., `csm-traits`) or package scope (e.g., `cli-npm`), also add
  the corresponding scope to `commitlint.config.cjs` scope-enum list.
- **Jules bot commits may not follow conventional format**: Added an ignore rule
  for PR merge commits that have no conventional prefix. This prevents CI failures
  on main after bot-authored PRs are merged.
- **Validate the full branch range**, not just the last commit: CI runs
  `commitlint --from base --to head`. A bad early commit fails the PR after later
  good commits land. Use `npx commitlint --from origin/main --to HEAD --verbose`.
- **Planning scopes**: `plans`, `goap`, `agents` are valid; avoid inventing scopes
  (`docs(plans)` failed until `plans` was added — prefer `docs:` or `docs(plans)`
  with the enum updated first).
- **Subject-case**: do not start subjects with acronyms in UpperCase (`TTL lifecycle`
  fails; use `ttl cleanup`).

### Wave 32 CI / correctness lessons (PR #518)
- **wasm-pack `--out-dir` is crate-relative** when building `crates/csm-wasm`; use
  absolute paths in CI/release or artifacts land under `crates/csm-wasm/...`.
- **NEON early-return leaves trailing parallel fallbacks unreachable on aarch64**
  under `-D warnings`; cfg-gate fallbacks.
- **Mutation timeouts ≠ kills** (ADR-0095); stub modules need excludes or real tests.
- **TTL `timeout(handle)` detaches on expiry** — always `abort()` on deadline.
- **Absence short-circuit must not starve keyword-only** and must invalidate on inject.
- See `.agents/skills/github-ci-guardrails/references/ci-pitfalls-wave32.md`.

### Supply Chain Advisory Discipline
- **Run `cargo deny check` before releases and after dependency upgrades**:
  New advisories can surface at any time. The `deny.toml` ignore list must be
  actively maintained — either upgrade affected deps or add documented ignore entries.
- **Simple upgrades first**: `cargo update -p <package>` often resolves advisories
  without any code changes (e.g., crossbeam-epoch 0.9.18→0.9.20, anyhow 1.0.102→1.0.103).
