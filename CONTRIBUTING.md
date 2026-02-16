# Contributing

## Development Setup

```bash
# Install Rust 1.82+
rustup install 1.82.0

# Install cargo components
rustup component add rustfmt clippy
```

## Code Standards

- Format: `cargo fmt`
- Lint: `cargo clippy -- -D warnings`
- Max 500 LOC per source file
- All public APIs return `Result<T, Error>`

## Testing

```bash
cargo test --all-features
cargo bench
```

## Pull Request Process

1. Update documentation if needed
2. Ensure CI passes
3. Update CHANGELOG.md if applicable
