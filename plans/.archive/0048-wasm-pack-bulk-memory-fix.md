# ADR-0048: Fix wasm-pack Build Bulk Memory Error

## Status

**Accepted** | **Implemented**

## Date

2026-02-27

## Context

Running `wasm-pack build --target web --scope d-o-hub` fails with wasm-opt validation errors:

```
[wasm-validator error in function 2] unexpected false: Bulk memory operations require bulk memory [--enable-bulk-memory], on 
(memory.copy ...)
```

This occurs because:
1. Rust's standard library (liballoc) uses bulk memory operations in release mode
2. wasm-pack runs wasm-opt to optimize the binary but doesn't pass the required flags
3. The WASM validator rejects the optimized binary without `--enable-bulk-memory`

The existing npm-publish.yml had `WASM_OPT_FLAGS` environment variable set, but this wasn't being picked up by wasm-pack 0.14.0 properly.

## Decision

Add wasm-pack configuration to Cargo.toml to pass the required wasm-opt flags:

```toml
[package.metadata.wasm-pack.profile.release]
wasm-opt = ["-O", "--enable-bulk-memory", "--enable-sign-ext"]
```

This ensures:
1. wasm-opt runs with optimization level `-O`
2. Bulk memory operations are enabled (`--enable-bulk-memory`)
3. Sign extension operations are enabled (`--enable-sign-ext`)

## Consequences

### Positive
- wasm-pack build now succeeds locally and in CI
- Binary size remains under 500KB limit (238KB)
- npm publishing workflow will work correctly

### Negative
- None

## Implementation

1. Added `[package.metadata.wasm-pack.profile.release]` section to Cargo.toml
2. Verified build succeeds: `wasm-pack build --target web --scope d-o-hub`
3. Confirmed binary size: 238KB (well under 500KB limit)

## References

- wasm-pack documentation: https://rustwasm.github.io/wasm-pack/
- wasm-opt flags: https://github.com/WebAssembly/binaryen
- Previous fix attempt in ADR-0046
