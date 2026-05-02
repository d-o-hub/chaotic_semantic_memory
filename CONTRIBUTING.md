# Contributing

## Development Setup

```bash
# Install Rust 1.85+ (edition 2024)
rustup install stable

# Install components
rustup component add rustfmt clippy

# Install WASM target (optional)
rustup target add wasm32-unknown-unknown

# Set up git hooks
scripts/setup-hooks.sh
```

## Code Standards

- Format: `cargo fmt`
- Lint: `cargo clippy --all-targets --all-features -- -D warnings`
- Strict Compliance: The linter must output zero warnings or errors. No exceptions for unused variables or functions; they must be removed or properly utilized.
- Max 500 LOC per source file
- All public APIs return `Result<T, Error>`
- Use `libsql` (never `turso-client`)
- Gate WASM threading with `#[cfg(not(target_arch = "wasm32"))]`
- No hardcoded magic numbers — use named constants

## Commit Conventions

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

<body>
```

| Type | Purpose | Version Bump |
|------|---------|-------------|
| `feat` | New feature | Minor |
| `fix` | Bug fix | Patch |
| `perf` | Performance | Patch |
| `docs` | Documentation | None |
| `test` | Testing | None |
| `chore` | Maintenance | None |
| `ci` | CI/CD | None |

Breaking changes: add `!` after type (e.g., `feat!:`) or include `BREAKING CHANGE:` in body.

## Validation

Run all gates before committing:

```bash
scripts/validate.sh
```

Or individually:

```bash
cargo check --all-targets --all-features
cargo test --all-features
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
```

## Testing

```bash
# All tests
cargo test --all-features

# Benchmarks
cargo bench --bench benchmark

# Mutation testing
scripts/mutation_test.sh fast

# Property-based tests
cargo test --test property_based
```

## Pull Request Process

1. Create a feature branch from `main`
2. Follow commit conventions above
3. Run `scripts/validate.sh` locally
4. Update `CHANGELOG.md` under `[Unreleased]` if applicable
5. Open PR — CI will run automatically
6. Ensure all CI checks pass

## Release Process

Releases are managed via `scripts/release-manager.sh`:

```bash
# Validate release readiness
scripts/release-manager.sh validate

# Prepare release (bump version, update changelog)
scripts/release-manager.sh prepare 0.2.0

# Publish (tag, push, create GitHub release)
scripts/release-manager.sh publish 0.2.0

# Or do everything at once
scripts/release-manager.sh full 0.2.0
```

See [ADR-0042](plans/adr/0042-release-automation-v010.md) for release automation details.

## Security

- Never commit API tokens or secrets
- Report vulnerabilities via [GitHub Security Advisories](https://github.com/d-o-hub/chaotic_semantic_memory/security/advisories/new)
- See [SECURITY.md](SECURITY.md) for full policy
