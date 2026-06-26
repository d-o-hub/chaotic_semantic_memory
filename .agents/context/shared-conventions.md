# d-o-hub Shared Conventions

Cross-repo conventions for all d-o-hub organization repositories.

## Commit Format

Conventional Commits: `type(scope): description`

| Type | Purpose |
|------|---------|
| feat | New feature |
| fix | Bug fix |
| perf | Performance improvement |
| refactor | Code restructuring (no behavior change) |
| test | Adding or updating tests |
| chore | Maintenance, dependencies |
| docs | Documentation only |
| ci | CI/CD configuration |

## Branch Naming

Pattern: `type/scope-description`

Examples:
- `feat/simd-chaotic-lsh`
- `fix/ci-mutation-gate`
- `test/inline-tests-clippy-config`
- `refactor/persistence-fk`

## PR Requirements

- All CI checks must pass before merge
- Squash merge preferred
- PR title follows conventional commit format (`type(scope): summary`)
- Description must include:
  - Summary of changes
  - What changed (files, modules)
  - What was tested (commands run, scenarios validated)
- Never push directly to `main`/`master`

## Quality Thresholds

| Metric | Threshold |
|--------|-----------|
| Source file length | ≤ 500 LOC |
| Test:source ratio | ≥ 90% |
| Mutation kill rate | ≥ 85% |
| Clippy warnings | 0 (`-D warnings`) |
| Format diffs | 0 (`cargo fmt --check`) |

## CI Gates

All PRs must pass these gates before merge:

1. `cargo check --quiet`
2. `cargo test --all-features --quiet`
3. `cargo fmt --check --quiet`
4. `cargo clippy --quiet -- -D warnings`
5. WASM size gate (library ≤ target threshold)
6. `scripts/mutation_test.sh fast` (kill rate ≥ 85%)

## GOAP State Management

- `action_last_completed` must appear **exactly once** in `plans/GOAP_STATE.md`
- Valid action status values: `queued`, `in_progress`, `complete`, `blocked`, `deferred`, `delegated`
- Update `plans/GOAP_STATE.md` after each completed action
- Use `delegated` + `jules_issue: <num>` when handing work to Jules

Verification:
```bash
grep -c '^  action_last_completed' plans/GOAP_STATE.md  # must output: 1
```

## ADR Process

- Create an ADR for non-trivial architectural decisions
- File location: `plans/adr/NNNN-title.md`
- Register every ADR in `plans/ADR_REGISTRY.md`
- Verify parity: `./scripts/check-adr-parity.sh` (exits 0 when in sync)

## Security

- Pin Rust dependencies to exact versions in `Cargo.toml` (no open ranges)
- Pin GitHub Actions to full SHA (not tags)
- Prefer well-known, actively maintained packages
- Flag unusual dependency names (typosquatting risk)
- Never commit secrets — reference by key name, not value
