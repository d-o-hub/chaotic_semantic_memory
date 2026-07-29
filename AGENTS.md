# AGENTS.md - Chaotic Semantic Memory

## Mission
Build and maintain `chaotic_semantic_memory` as a production Rust crate for AI memory systems.

---

## Workflow (REQUIRED for Every Session)

**This workflow MUST be followed for every coding task. Skipping steps causes regressions.**

### Phase 1: Context Load (WHAT)

1. **Read state files first** — Load current world state before any work:
   - `@plans/GOAP_STATE.md` — Current world state, completed phases, module LOC
   - `@plans/ACTIONS.md` — Queued actions and their preconditions

2. **Review ALL uncommitted changes** — Never start implementation without knowing the full scope:

   ```bash
   git status --short           # List ALL modified/untracked files
   git diff HEAD                # Review content of pending changes
   ```

   - If unrelated changes exist, either commit them first or explicitly scope them out
   - Document which pending changes are intentionally excluded from this session

3. **Run proactive LOC gate check** — Pre-existing violations cascade on commit, wasting iterations:

   ```bash
   find src crates -name '*.rs' -not -path '*/target/*' -exec wc -l {} + | sort -rn | head -20
   # Verify every file is ≤ 500 LOC. Fix violations BEFORE starting work.
   ```

4. **Understand the codebase structure** — Know where files live before editing:
   - Core modules: `src/singularity.rs`, `src/reservoir.rs`, `src/framework.rs`
   - Persistence: `src/persistence.rs`, `src/persistence_ops.rs`
   - Retrieval: `src/retrieval/bm25.rs`, `src/retrieval/hybrid.rs`
   - Bridge: `src/semantic_bridge.rs`, `src/bridge_retrieval.rs`
   - CLI: `src/cli/commands/*.rs`

5. **Check CI status** — Verify baseline before changes:

   ```bash
   gh run list --workflow=ci.yml --limit 3
   ```

5b. **Check open PR merge conflicts** — When triaging multiple PRs, check
   conflict status FIRST (before CI). A `CONFLICTING` PR cannot merge regardless
   of CI status:

   ```bash
   gh pr list --state open --json number,title,mergeable \
     --jq '.[] | "\(.number): \(.mergeable) — \(.title)"'
   ```

6. **Verify built binary, not stale install** — `~/.local/bin/csm` (or any global
   install) may lag the source tree by multiple releases. Before claiming a CLI
   surface is missing a command, always confirm against a fresh build:

   ```bash
   cargo build --bin csm --features cli --quiet
   ./target/debug/csm --help                     # source truth
   csm --help | head -1                          # installed truth (may be stale)
   ```

   If they disagree, the gap is in distribution, not source.

7. **Verify ADR registry ↔ disk parity** — Stops state drift between
   `plans/ADR_REGISTRY.md` and the actual ADR files:

   ```bash
   ./scripts/check-adr-parity.sh                 # exits 0 when in sync
   ```

### Phase 2: Planning (WHY)

6. **Plan before implementing** — For non-trivial tasks (3+ steps):
   - Explore codebase before proposing changes
   - Identify affected files and dependencies
   - Document approach in `plans/` directory
   - Get user approval before implementation
   - **TRIZ Integration**: Use `triz-analysis` skill for architectural decisions
   - **Problem Solving**: Use `triz-solver` skill when stuck on complex problems

7. **Use parallel execution for complex changes** — For multi-file tasks:
   - Create task list for each subtask
   - Spawn specialized workers with clear prompts
   - Assign tasks and monitor progress
   - Clean up resources after completion

### Phase 3: Implementation (HOW)

8. **Edit files with precision** — Never bulk-edit without reading first:
   - Read before editing — understand existing code
   - Match existing style, naming, patterns
   - Preserve comments and docstrings unless explicitly removing
   - **Performance claims require benchmark evidence** — Any PR with a perf claim
     in its title or body MUST attach `criterion` output or flamegraph results.
     Use the `benchmarking-perf` skill. No evidence = PR is not review-ready.

9. **Run validation gates after changes** — Verify before proceeding:

   ```bash
   cargo check --quiet                              # Compile check
   cargo test --all-features --quiet                # Unit + integration tests
   cargo fmt --check --quiet                        # Format check
   cargo clippy --quiet -- -D warnings              # Lint check
   ./scripts/check-adr-parity.sh                    # ADR registry ↔ disk parity
   shellcheck scripts/*.sh                          # Shell hygiene
   ```

   When touching CLI surface (`src/cli/**` or `src/bin/csm.rs`), also:

   ```bash
   cargo test --test cli_parity --features cli      # 22-command surface lock
   ```

10. **Coverage validation** — Ensure test coverage meets target:

    ```bash
    # Calculate test:source ratio
    test_loc=$(wc -l tests/*.rs | tail -1 | awk '{print $1}')
    src_loc=$(wc -l src/*.rs src/**/*.rs | tail -1 | awk '{print $1}')
    ratio=$((test_loc * 100 / src_loc))
    # Target: >= 90% coverage
    ```

11. **Real usage validation** — Test production scenarios. Always use the
    freshly built binary (`./target/debug/csm`), never the globally installed
    `csm` which may be multiple releases behind:

    ```bash
    CSM=./target/debug/csm   # NOT the stale ~/.local/bin/csm
    $CSM inject test-1 --database /tmp/validate.db
    $CSM probe test-1 -k 5 --database /tmp/validate.db
    $CSM export -o /tmp/validate.json --database /tmp/validate.db
    $CSM import /tmp/validate.json --database /tmp/validate.db
    rm /tmp/validate.db /tmp/validate.json

    # Skill-memory integration
    ls -la .agents/csm-memory/skill-memory.db  # Verify db exists
    ```

12. **Update state after completion** — Record what changed:
   - Update `plans/GOAP_STATE.md`: `action_last_completed`, module LOC, test counts.
     `action_last_completed` MUST appear **exactly once** in the file (YAML
     last-key-wins makes earlier duplicates silently dead).
     Check: `grep -c '^  action_last_completed' plans/GOAP_STATE.md` → `1`.
   - Update `plans/ACTIONS.md` status. Valid status values:
     `queued`, `in_progress`, `complete`, `blocked`, `deferred`
   - Add learnings to `progress/LEARNINGS.md` if new patterns discovered.

### Phase 4: Verification (Compound Engineering)

13. **Run full validation before claiming completion**:
   ```bash
   ./scripts/validate.sh                    # All gates in one command
   ```

14. **If errors occur, encode corrections** — Compound engineering principle:
    - Fix the immediate error
    - Add rule/constraint to prevent recurrence
    - Update AGENTS.md or hard-constraints.md if systemic

### Phase 5: Atomic Commit & CI Gate (GOAP Orchestration)

15. **Create feature branch FIRST** — `main` is protected, never commit directly:
    ```bash
    git checkout -b <type>/<scope>-<description>
    # Examples: test/inline-tests-clippy-config, fix/persistence-fk, feat/reservoir-simd
    ```

16. **Atomic commits** — One logical change per commit, never mix unrelated changes:
    ```bash
    git add src/singularity.rs src/singularity_cache.rs
    git commit -m "feat(singularity): add similarity cache"
    ```

17. **Push branch and create PR** — Never push directly to `main`:
    ```bash
    git push origin <branch>
    gh pr create --title "<type>(<scope>): <summary>" --body "..."
    gh pr checks --watch  # Wait for CI to pass
    ```

18. **Merge after CI passes** — Only merge when all checks are green:
    ```bash
    gh pr merge  # Squash merge preferred
    ```

    **⚠️ NEVER use `gh pr merge --auto` when merging multiple PRs.**
    This repo requires "up to date with base". Auto-merge on multiple PRs
    creates a rebase loop (merge A → B is stale → auto-merge cancelled).
    Instead: merge one PR → rebase next → wait for CI → merge → repeat.

    **Jules / multi-PR extras** (see `.agents/skills/git-workflow/SKILL.md`):
    - Close empty research PRs (0 files) without CI thrash.
    - Before merge, `git diff --stat origin/main...HEAD` to catch bot force-push
      reverts of already-merged work.
    - Commitlint: full range `npx commitlint --from origin/main --to HEAD`.
    - **Never delete rationale comments** — Comments explaining *why* a design
      decision was made (e.g. cargo-mutants suppression strategy, IEEE 754
      semantics) are load-bearing. Rewriting them to describe the current PR
      erases institutional knowledge. Preserve or extend; never replace.
    - **Never inline test variables** — Named variables (`let d1_score = ...`)
      and per-assert formula comments (`// d1: 0.6 * 1.0 + 0.4 * 1.0 = 1.0`)
      are required for on-call debuggability. Do not collapse into chained
      one-liners or compound `&&` asserts. Each `assert!` must target a single
      value so failures are unambiguous.

19. **Fix ALL issues (including pre-existing)** — CI must pass completely:
    - New failures: Fix immediately
    - Pre-existing warnings: Fix before claiming completion
    - Use `goap-planning` skill to track fix actions in GOAP_STATE
    - Update `action_last_completed` and `world_state` after each fix

20. **Document in GOAP_STATE** — Record completion state:
    ```yaml
    world_state:
      action_last_completed: <action_name>
      ci_all_checks_passed: true
      tests_count: <new_count>
    ```

---

## Session Checklist

Before starting any task, verify:
- [ ] GOAP_STATE.md loaded — know current state
- [ ] ALL uncommitted changes reviewed via `git status --short`
- [ ] **LOC gate pre-check**: all source files ≤ 500 LOC (`find src crates -name '*.rs' -not -path '*/target/*' -exec wc -l {} + | sort -rn | head -20`)
- [ ] **PR conflict check**: `scripts/pr-triage.sh` or `gh pr list --state open --json number,mergeable --jq '.[] | select(.mergeable == "CONFLICTING")'`
- [ ] Hard constraints understood — spectral radius [0.9, 1.1]
- [ ] CI baseline confirmed via `gh run list`

Before completing any task, verify:
- [ ] **Branch created (NOT main)** — never push directly to protected branch
- [ ] **Commitlint full range** — `npx commitlint --from origin/main --to HEAD --verbose`
- [ ] **PR created and CI passing** — merge only after green checks
- [ ] All validation gates pass (check, test, fmt, clippy)
- [ ] **Harness sensors green** — `./scripts/harness-check.sh all` (fmt → clippy → deny → test → arch); fix each `❌ HARNESS VIOLATION` before proceeding
- [ ] **CI pitfall scan** when touching mutation/TTL/CLI/Jules PRs — see
      `.agents/skills/github-ci-guardrails/references/ci-pitfalls-pr-triage.md`
- [ ] **Coverage gate** — test:source ratio >= 90% (or improving)
- [ ] **Real usage validated** — CLI workflow, skill-memory db, file persistence
- [ ] CI workflow passes
- [ ] GitHub Actions warnings/issues checked via `gh run view`
- [ ] Pre-existing warnings fixed (not just new issues)
- [ ] GOAP_STATE.md updated with `action_last_completed`
- [ ] Learnings captured if new patterns discovered

---

## 8 Core Rules

1. **Always read before editing** — Never guess file contents.

2. **Stay under context limits** — Each instruction must earn its place.

3. **Hooks for deterministic enforcement** — Validation gates are mandatory.

4. **Use `@imports` for modularity** — Reference files via `@path/to/file` syntax.

5. **Plan before implementing** — For tasks with 3+ steps.

6. **Update monthly, encode errors immediately** — Every correction becomes a rule.

7. **Reference, don't duplicate** — Point to source files, don't restate contents.

8. **Never push directly to `main`** — Create branch → commit → PR → merge after CI passes.

---

## DeepSource Parity (Coding Standards)

These patterns mirror DeepSource's Rust analyzer rules. Violations block CI.

### DO: Construct directly in `Default::default()`

```rust
// ✅ CORRECT: default() constructs the struct directly; new() delegates to default()
impl Default for FrameworkBuilder {
    fn default() -> Self {
        Self {
            config: FrameworkConfig::default(),
            db_path: None,
            // ...
        }
    }
}

impl FrameworkBuilder {
    pub fn new() -> Self {
        Self::default()
    }
}

// ❌ WRONG: default() calling Self::new() triggers DeepSource BUG_RISK
impl Default for FrameworkBuilder {
    fn default() -> Self {
        Self::new()  // DeepSource: "Found call returning Self in default()"
    }
}
```

### DO: Use `.map_or()` / `.is_some_and()` instead of `.map().unwrap_or()`

```rust
// ✅ CORRECT
concepts.get(id).is_some_and(|c| filter.matches(&c.metadata))
value.map_or_else(|| default(), |s| s.to_string())

// ❌ WRONG: triggers DeepSource ANTI_PATTERN + clippy::map_unwrap_or
concepts.get(id).map(|c| filter.matches(&c.metadata)).unwrap_or(false)
value.map(|s| s.to_string()).unwrap_or_else(|| default())
```

### Clippy Lints Enforcing These

| Pattern | Clippy Lint | Active? |
|---------|-------------|---------|
| `.map(f).unwrap_or(g)` | `clippy::map_unwrap_or` (pedantic) | ✅ Promoted to `warn` in Cargo.toml |
| `.map_or(false, f)` | `clippy::unnecessary_map_or` (in `all`) | ✅ Implied by `-D warnings` |
| `Self::new()` in `default()` | No clippy equivalent | 📋 Documented above |
| `unwrap()`/`expect()`/`panic!()` in library | `clippy::unwrap_used`, `clippy::expect_used`, `clippy::panic` | ✅ `warn` in workspace lints |

### Lint Policy: unwrap/expect/panic

- **Workspace lints** (`Cargo.toml`): `unwrap_used = "warn"`, `expect_used = "warn"`, `panic = "warn"`
- **Test exemption** (`.clippy.toml`): `allow-unwrap-in-tests = true`, `allow-expect-in-tests = true`, `allow-panic-in-tests = true`
- **Production allows**: Use `#[allow(clippy::expect_used)]` with a justification comment for infallible operations (lock acquisition, static construction)
- **CI enforcement**: `cargo clippy -- -D warnings` promotes all warnings to errors

---

## Hard Constraints
See: [agents-docs/hard-constraints.md](agents-docs/hard-constraints.md)

---

## Release Safety Requirements

**CRITICAL: Never release with failing CI. The release workflow now has a guardrail that waits for CI to pass.**

### Artifact Selection (REQUIRED)

Before validating, installing, or publishing, identify the correct channel:
- **Rust Library:** `chaotic_semantic_memory` (crates.io / cargo)
- **JS/WASM Library:** `@d-o-hub/chaotic_semantic_memory` (npm WASM)
- **CLI Tool:** `@d-o-hub/csm` (npm CLI)

Refer to the `dist-channel-selection` skill for canonical commands.

### Pre-Release Checklist (MANDATORY)

1. **Verify CI passes on all platforms**:
   ```bash
   gh run list --workflow=ci.yml --limit 3
   gh run view --log  # Check all jobs: macos-arm64, windows-x64, linux
   ```

2. **Ensure Cargo.lock is synchronized**:
   ```bash
   cargo build --release  # Regenerates Cargo.lock after version bump
   git add Cargo.lock     # Must be committed with version changes
   ```

3. **Check existing releases**:
   ```bash
   gh release list --limit 5
   gh release view --json tagName,isLatest
   ```

4. **Validate changelog entry exists**:
   ```bash
   grep -q "^## \[${VERSION}\]" CHANGELOG.md
   ```

5. **Verify crates.io name availability** (FIRST PUBLISH ONLY):
   ```bash
   # Check if workspace crate names are already taken by unrelated projects
   for crate in csm-chaos csm-core-lib csm-traits csm-embedding csm-memory csm-retrieval csm-persistence; do
     echo -n "$crate: "
     cargo search "$crate" 2>/dev/null | head -1
   done
   # If names are taken, rename crates before first publish (e.g., csm-core → csm-core-lib)
   ```

6. **Check crates.io environment wait timer**:
   ```bash
   # The crates.io environment may have a wait timer (default 15min)
   # Check pending deployments before re-running failed releases
   gh api repos/{owner}/{repo}/actions/runs/{run_id}/pending_deployments
   # If wait_timer > 0, either wait or remove timer in GitHub Settings → Environments → crates.io
   ```

### Version Bump Workflow

1. Update `Cargo.toml` version
2. Update `wasm/package.json` version
3. Update `CHANGELOG.md` with new section
4. Run `cargo build --release` to sync Cargo.lock
5. Commit all version files together (atomic)
6. Push and wait for CI to pass
7. Only then create tag/release

### Platform-Specific Considerations

- **macOS arm64**: NEON SIMD intrinsics require explicit unsafe blocks
- **Windows x64**: CI uses `--locked` flag, Cargo.lock must match Cargo.toml
- **WASM**: Size gate checks library (~870KB), not CLI binary (~5KB)

### Reference Files

- `.github/workflows/release.yml` — Has `wait-for-ci` guardrail job
- `.agents/skills/release-management/` — Full release skill
- `scripts/validate.sh` — Pre-commit validation gates

### Common Deployment Failures

1. **crates.io name conflicts**: Workspace crate names may be taken by unrelated projects. Check `cargo search` before first publish. Rename crates if needed (e.g., `csm-core` → `csm-core-lib`).

2. **Environment wait timer**: The `crates.io` GitHub environment may have a wait timer (typically 15 minutes). This blocks `publish-crates` job until the timer expires. Remove the timer in GitHub Settings → Environments → crates.io if not needed.

3. **Token ownership**: The `CARGO_REGISTRY_TOKEN` must belong to a user who owns all crates on crates.io. If a crate name is taken by another project, the token won't have permission to publish. Use `cargo owner --list <crate>` to verify ownership.

4. **Recovery dispatch**: Use `gh workflow run release.yml -f recover=true` to retry failed releases for existing tags. The workflow is idempotent — it skips already-published artifacts.

---

## PR & CI Workflow

1. **Consolidate dependent changes** — When fixing CI issues across multiple dependent changes, consolidate into a single PR rather than creating separate PRs with circular dependencies. Each PR must be independently CI-green.

2. **Analyze dependencies before splitting** — Before creating multiple PRs, analyze dependency relationships between changes. If changes depend on each other to pass CI, they belong in one PR or must be ordered with a clear base chain (A merges → B rebases on main → B merges → C rebases → ...).

3. **Never use `gh pr merge --auto` on multiple PRs** — This repo requires "up to date with base". Auto-merge on multiple PRs creates a rebase loop. Instead: merge one → rebase next → wait for CI → merge → repeat.

---

## Workflow Patterns

1. **Analysis → Synthesis → Action** — When executing recommendations from a codebase analysis, use parallel task agents to explore different areas simultaneously, then synthesize findings before making changes.

2. **Pre-flight dependency graph** — Before proposing a PR strategy, map which files/crates depend on which changes to compile and pass tests. Draw the graph, then decide the split.

---
## Key Files
**Core**: `src/singularity.rs`, `src/reservoir.rs`, `src/reservoir_inertial.rs`, `src/framework.rs`, `src/persistence.rs`
**Bridge**: `src/semantic_bridge.rs`, `src/bridge_retrieval.rs`
**Retrieval**: `src/retrieval/bm25.rs`, `src/retrieval/hybrid.rs`, `src/singularity_retrieval.rs`
**CLI**: `src/cli/commands/query.rs`, `src/cli/commands/index_dir.rs`
**State**: `plans/GOAP_STATE.md`, `plans/ACTIONS.md`

## Skills (32 Total)
**Core**: `rust-development`, `testing-validation`, `goap-planning`, `goap-orchestrator`, `adr-creation`, `github-ci-guardrails`, `git-workflow`, `release-management`, `dist-channel-selection`, `benchmarking-perf`, `debugging-reservoir`, `skill-memory-internal`, `memory-lifecycle-verification`, `turso-memory-verification`, `drawio`, `npm-trusted-publishers`

**Swarm**: `swarm-testing-quality`, `swarm-performance`, `swarm-observability`, `swarm-advanced-features`, `analysis-swarm`

**Workflow**: `learn`, `task-decomposition`, `shell-script-quality`, `jules-orchestration`

**Automation**: `self-fix-loop`, `iterative-refinement`, `skill-creator`, `skill-evaluator`, `codacy`

**TRIZ**: `triz-analysis`, `triz-solver`

## External References
- [agents-docs/hard-constraints.md](agents-docs/hard-constraints.md) — LOC limits, spectral radius
- [agents-docs/accuracy-guardrails.md](agents-docs/accuracy-guardrails.md) — API verification
- [agents-docs/quick-reference.md](agents-docs/quick-reference.md) — Commands
- [agents-docs/self-learning-patterns.md](agents-docs/self-learning-patterns.md) — Compound engineering
