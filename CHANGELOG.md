# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- CLI crate (`csm` binary) with inject, probe, associate, export, import, and completions commands
- Shell completion generation for bash, zsh, fish, and powershell
- `ConceptBuilder` module for ergonomic concept construction
- `FrameworkBuilder` and `FrameworkConfig` as dedicated types
- Release management skill (`.agents/skills/release-management/`)
- GitHub Pages documentation workflow with mdBook
- crates.io Trusted Publishing workflow (OIDC-based)
- npm publishing workflow with provenance for WASM bindings

### Changed
- Upgraded to Rust Edition 2024 (MSRV 1.85)
- Added `#[must_use]` annotations to constructors
- Improved unsafe block documentation in SIMD operations
- Replaced format! JSON with serde_json in CLI
- Updated MSRV from 1.82 to 1.85

### Fixed
- CLI exit codes now correctly map to error types (ADR-0032)
- CLI JSON output escaping safety (ADR-0032)
- Async lock safety across await points (ADR-0031)
- WASM Reflect::set panic safety (ADR-0033)
- Query cache memory blow-up prevention (ADR-0035)
- LOC gate now includes src/cli/ and src/bin/ (ADR-0036)

### Security
- Added `SECURITY.md` with vulnerability reporting process
- Added `CODE_OF_CONDUCT.md` (Contributor Covenant 2.1)
- Updated CI workflow with security permissions and concurrency controls
- Trusted Publishing eliminates need for long-lived API tokens

## [0.1.0] - 2026-02-17

### Added
- Initial release of `chaotic_semantic_memory`
- Hyperdimensional vector operations with SIMD support
- Echo state network reservoir for temporal processing
- Semantic memory framework with concept storage
- Turso/libSQL persistence layer
- WASM target support
- Property-based testing with proptest
- Fuzzing targets for critical paths
- Comprehensive benchmark suite
- Performance gates (< 100μs reservoir step)
- Memory footprint validation (< 12MB for 10M concepts)
- WASM binary size gate (< 500KiB)
- Schema migration support
- Export/import functionality (JSON + binary)
- Concept versioning
- Backup/restore operations
- Structured logging with tracing
- Metrics collection
- Connection pooling for Turso
- LRU concept cache
- Zero-allocation query cache

[unreleased]: https://github.com/d-o-hub/chaotic_semantic_memory/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.1.0
