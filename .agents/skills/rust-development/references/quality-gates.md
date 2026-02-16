# Quality Gates

Run before finalizing changes:

```bash
cargo check
cargo test --all-features
cargo fmt --check
cargo clippy -- -D warnings
```

Enforce source-file LOC limits and add missing tests before merge.
