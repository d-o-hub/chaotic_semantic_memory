# Contributing

## Development Setup

### Nix (Recommended)

This project provides a `flake.nix` for a reproducible development environment.

```bash
# Enter the development shell with all tools pinned (Rust, WASM, Node.js)
nix develop
```

### Manual Setup

```bash
# Install Rust 1.88+ (edition 2024)
rustup install stable

# Install components
rustup component add rustfmt clippy

# Install WASM target (optional)
rustup target add wasm32-unknown-unknown

# Set up git hooks
pip install pre-commit
pre-commit install
pre-commit install --hook-type commit-msg
pre-commit install --hook-type pre-push
```

## Code Standards

- Format: `cargo fmt`
- Lint: `cargo clippy --all-targets --all-features -- -D warnings`
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

## Profiling

To profile the application with `perf` or `flamegraph` while maintaining release-level optimizations, use the `profiling` profile:

```bash
# Build with profiling symbols preserved
cargo build --profile profiling

# Run with perf
perf record --call-graph dwarf ./target/profiling/csm
```

This profile inherits from `release` but preserves debug symbols (`debug = 1`) and disables stripping.

## Pull Request Process

1. Create a feature branch from `main`
2. Follow commit conventions above
3. Run `scripts/validate.sh` locally
4. Update `CHANGELOG.md` under `[Unreleased]` if applicable
5. Open PR — CI will run automatically
6. Ensure all CI checks pass

## Release Process

Releases are managed using `cargo-release`, `git-cliff`, and `cargo-dist`.

### 1. Preparation

Ensure your workspace is clean and you are on the `main` branch.

### 2. Validation

Run the validation gates to ensure the project is in a releasable state:

```bash
scripts/validate.sh
```

### 3. Release Dry-run

Verify the release process without making any changes:

```bash
cargo release [patch|minor|major] --dry-run
```

### 4. Execute Release

Perform the release. This will bump the version, update `CHANGELOG.md` using `git-cliff`, sync versions across the codebase, create a git tag, and push to GitHub.

```bash
cargo release [patch|minor|major] --execute
```

### 5. Distribution

Once the tag is pushed, GitHub Actions will automatically:
1. Build and publish workspace crates to crates.io.
2. Build and publish WASM bindings to npm.
3. Build and publish the CLI to npm.
4. Create a GitHub Release with binaries for all supported platforms (managed by `cargo-dist`).

See [ADR-0042](plans/adr/0042-release-automation-v010.md) for historical release automation details.

## Security

- Never commit API tokens or secrets
- Report vulnerabilities via [GitHub Security Advisories](https://github.com/d-o-hub/chaotic_semantic_memory/security/advisories/new)
- See [SECURITY.md](SECURITY.md) for full policy
