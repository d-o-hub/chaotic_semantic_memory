# Unmaintained Crates — Known Advisories

This file documents crates flagged by `cargo audit` as unmaintained.
These are **not security vulnerabilities** — they are warnings that the crate
maintainer is no longer actively updating the crate.

The `security-audit` job in `pre-release-gate.yml` runs `cargo audit` (without
`--deny warnings`) so these warnings remain **visible in CI logs** but do not
**block** the pre-release gate. If `cargo audit` ever exits non-zero, that
indicates a **new real vulnerability** that needs immediate attention.

---

## Current Advisories (2026-05-21)

### 1. bincode 1.3.3 — RUSTSEC-2025-0141

- **Status**: Unmaintained since 2025-12-16
- **Type**: Direct dependency
- **Used by**: `chaotic_semantic_memory` (direct), `libsql v0.9.30` (transitive)
- **Files**: `src/hyperdim_serde.rs`, `src/framework_ops.rs`, `src/wasm.rs`,
  `src/index/lsh.rs`, `src/index/hnsw.rs`, `src/export_payload.rs`
- **Migration plan**: Upgrade to bincode 2.x. Key API changes:
  - `bincode::serialize()` → `bincode::serde::encode_into_slice()` or `bincode::encode_to_vec()`
  - `bincode::deserialize()` → `bincode::serde::decode_from_slice()`
  - `bincode::DefaultOptions` → `bincode::config::standard()`
  - `with_limit()` → `bincode::config::Configuration::with_limit()`
- **Blockers**: `libsql v0.9.30` transitively depends on bincode 1.x;
  need to verify libsql >=0.10 supports bincode 2.x, or wait for upstream.
- **Estimated effort**: 4-6 hours (direct usage) + upstream coordination

### 2. number_prefix 0.4.0 — RUSTSEC-2025-0119

- **Status**: Unmaintained since 2025-11-17
- **Type**: Transitive dependency (not direct)
- **Used by**: Transitively through the dependency tree (was visible in
  older lock files; still flagged by advisory DB)
- **Migration plan**: None needed unless a direct dependency starts using it.
  If it reappears in a future `cargo update`, audit the chain and consider
  forking or replacing the upstream crate that pulls it in.
- **Estimated effort**: 0 (monitor only)

### 3. paste 1.0.15 — RUSTSEC-2024-0436

- **Status**: Unmaintained since 2024-10-07
- **Type**: Transitive dependency (not direct)
- **Used by**: Transitively — flags in advisory DB but `cargo tree -i paste`
  returns no matches in current dependency resolution
- **Migration plan**: Same as number_prefix — monitor. If it resurfaces after
  a `cargo update`, investigate the upstream crate and replace or fork.
- **Estimated effort**: 0 (monitor only)

---

## Process

1. **Monthly review**: Check `cargo audit` output for new advisories.
2. **New unmaintained crate**: Document here with migration plan.
3. **Crate migrated/removed**: Move to "Resolved" section below.
4. **New real vulnerability** (security, not unmaintained): Treat as P0 —
   `cargo audit --deny warnings` will catch this because it exits non-zero
   for vulnerability advisories even without `--deny warnings`.

---

## Resolved

None yet — all 3 advisories above are current as of 2026-05-21.
