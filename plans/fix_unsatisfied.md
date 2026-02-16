# Fix Plan: remove hardcoded config/magic numbers and stabilize APIs

1. Identify hard-coded numeric/config values in core modules.
2. Introduce centralized configuration constants and builder-driven options.
3. Replace unsafe/non-portable patterns and add wasm-safe/non-blocking guards.
4. Ensure Turso/object-store behavior uses configurable retry/cache/checkpoint sizing.
5. Update benchmarks/examples to consume config constants and avoid hidden literals.
6. Run full validation: cargo check, cargo test, cargo bench --bench benchmark, wasm build.
