---
name: rust-verification-loop
description: Run full Rust implementation verification loop with check, test, bench, wasm build, and example execution for production readiness.
---

# Rust Verification Loop

Use this skill for final production verification.

## Workflow
1. `cargo check`
2. `cargo test`
3. `cargo bench --bench benchmark`
4. `cargo build --target wasm32-unknown-unknown --release`
5. Run at least one working example end-to-end.

## References
- `references/commands.md`
