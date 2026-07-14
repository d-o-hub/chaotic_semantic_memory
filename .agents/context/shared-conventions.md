# d-o-hub Shared Conventions

Cross-repo conventions for the d-o-hub organization. Referenced by `AGENTS.md`
and consumable by derived repositories.

---

## Commit Format

[Conventional Commits](https://www.conventionalcommits.org/) required.

```
<type>(<scope>): <description>

[optional body]
[optional footer(s)]
```

### Types

| Type | Purpose | Version Bump |
|------|---------|--------------|
| `feat` | New feature | Minor |
| `fix` | Bug fix | Patch |
| `perf` | Performance improvement | Patch |
| `refactor` | Code change (no feature/fix) | None |
| `docs` | Documentation only | None |
| `test` | Adding/fixing tests | None |
| `chore` | Maintenance, deps | None |
| `ci` | CI/CD changes | None |
| `build` | Build system changes | None |

### Breaking Changes

Add `!` after type/scope: `feat(framework)!: remove deprecated API`
Or include `BREAKING CHANGE:` footer.

### Scopes (per-repo)

Scopes are enforced via `commitlint.config.cjs` in each repo. Common scopes:
- Module names (`singularity`, `reservoir`, `framework`, `persistence`)
- Infrastructure (`ci`, `build`, `deps`, `release`)
- Cross-cutting (`docs`, `clippy`, `lints`, `workspace`)

---

## Branch Naming

```
<type>/<scope>-<description>
```

Examples:
- `feat/retrieval-bm25-scoring`
- `fix/persistence-fk-constraint`
- `perf/core-simd-hamming`
- `test/inline-tests-coverage`
- `chore/deps-update-tokio`

---

## Pull Request Requirements

1. **Branch from main** — never commit directly to `main`
2. **All CI checks must pass** — including:
   - `cargo check --all-features`
   - `cargo test --all-features`
   - `cargo fmt --check`
   - `cargo clippy -- -D warnings`
   - Codacy static analysis
   - CodeQL security scanning
   - Cross-platform builds (Linux, macOS, Windows)
3. **Squash merge** — preferred merge strategy for clean history
4. **Title follows conventional commit format** — enforced by commitlint
5. **Delete branch after merge** — keep remote clean

### PR Title Format

```
<type>(<scope>): <concise description under 70 chars>
```

### PR Description Structure

```markdown
## Summary
Brief description of what changed and why.

## Changes
- Bullet list of specific changes

## Testing
How the changes were verified.
```

---

## Quality Thresholds

| Gate | Threshold | Enforcement |
|------|-----------|-------------|
| Source file LOC | ≤ 500 lines | CI gate + pre-commit |
| Test:source ratio | ≥ 90% | Monitored |
| Clippy warnings | 0 (deny all) | CI blocks merge |
| Format | `cargo fmt` standard | CI blocks merge |
| Coverage regression | No decrease | Codacy quality gate |
| Mutation score | ≥ 85% | CI gate (mutation-test job) |

---

## Dependency Policy

- Pin exact versions in `Cargo.lock` (committed to repo)
- Use workspace dependency inheritance (`[workspace.dependencies]`)
- Dependabot enabled for automated updates
- `cargo deny` enforces:
  - License allowlist (MIT, Apache-2.0, BSD-2/3, ISC, etc.)
  - Advisory database (RUSTSEC)
  - Source restrictions (no unknown registries/git)
- Prefer well-maintained crates; document unmaintained deps with justification

---

## Security Standards

- Parameterized SQL queries (never string interpolation)
- Input validation at API boundaries (size limits, format checks)
- No `unwrap()`/`expect()`/`panic!()` in library code (warn lint, test exemption)
- `unsafe` blocks require `// SAFETY:` comments and are restricted to SIMD modules
- File path validation (no path traversal)
- Secrets never logged or returned in responses

---

## Architecture Decision Records (ADRs)

For significant design decisions:

1. Create `plans/adr/NNNN-<slug>.md` (next sequential number)
2. Register in `plans/ADR_REGISTRY.md`
3. Format: Status, Context, Decision, Consequences
4. ADRs are immutable once accepted; supersede with new ADR if needed

---

## CI/CD Conventions

- Primary CI: GitHub Actions (`.github/workflows/ci.yml`)
- Release: Tag-triggered workflow with `wait-for-ci` guard
- Platform matrix: linux-x64, linux-arm64, macos-arm64, macos-x64, windows-x64
- WASM: Separate build + size gate
- Never use `gh pr merge --auto` when merging multiple PRs (rebase loop risk)
- Merge one PR → rebase next → wait for CI → merge → repeat

---

## GOAP Planning System

State tracked in `plans/GOAP_STATE.md` and `plans/ACTIONS.md`:
- `GOAP_STATE.md`: Current world state (boolean flags, metrics)
- `ACTIONS.md`: Action queue with preconditions, effects, status

Valid action statuses: `queued`, `in_progress`, `complete`, `blocked`, `deferred`

Update after every completed task:
- Set `action_last_completed` (exactly once in file)
- Update relevant world state flags
- Mark action status as `complete`
