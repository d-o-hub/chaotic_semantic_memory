# ADR-0038: Cargo.toml Modernization

## Status

Accepted (backfilled 2026-05-01) - Wave 10 Complete

## Context

Cargo.toml outdated for 2026 standards:
- Edition 2021 (not 2024)
- Missing crates.io metadata
- Dependency versions not pinned
- CLI deps not gated for WASM

## Decision

Implement **Cargo.toml modernization**.

**Deliverables:**
- Edition 2024 with MSRV 1.85
- crates.io metadata (description, license, repository, keywords, categories)
- Dependency versions pinned to specific patches
- CLI deps gated: target.'cfg(not(target_arch = "wasm32"))'.dependencies

## Consequences

### Positive
- crates.io-ready package
- Edition 2024 language features
- Reproducible builds (pinned versions)
- WASM builds skip CLI deps

### Negative
- MSRV 1.85 may exclude some users
- Edition 2024 migration effort
- Pinned versions require updates

## Implementation

- File: Cargo.toml
- Edition: 2024, rust-version: "1.85"
- Metadata: description, license, repository, keywords, categories, include

## Sources

- ACTIONS.md lines 1529-1627 (Phase 24 actions)
- ADR_REGISTRY.md: Cargo.toml Modernization
- Cargo.toml: current state