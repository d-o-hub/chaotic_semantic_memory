# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.2] - 2026-02-28

### Fixed
- npm package name: corrected `@d-o-hub/chaotic-semantic-memory` → `@d-o-hub/chaotic_semantic_memory` (underscore)
- npm publishing: v0.1.2 now published with OIDC provenance via GitHub Actions
- Updated workflow to use Node.js 24 with npm fallback for token authentication

### Changed
- npm workflow now uses trusted publishing (OIDC) when configured

## [0.1.1] - 2026-02-27

### Added
- ADR-0048: wasm-pack bulk memory fix configuration
- `#[instrument(err)]` on framework and framework_ops fallible functions
- Tracing instrumentation for reservoir and persistence operations

### Fixed
- wasm-pack build failing due to missing `--enable-bulk-memory` wasm-opt flag
- npm publishing workflow (ADR-0046)
- Various dependency updates

### Changed
- Optimized find_similar and to_hypervector with parallelization

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

### Changed
- CLI crate (`csm` binary) with inject, probe, associate, export, import, and completions commands
- Shell completion generation for bash, zsh, fish, and powershell
- `ConceptBuilder` module for ergonomic concept construction
- `FrameworkBuilder` and `FrameworkConfig` as dedicated types
- Release management skill (`.agents/skills/release-management/`)
- GitHub Pages documentation workflow with mdBook
- crates.io Trusted Publishing workflow (OIDC-based)
- npm publishing workflow with provenance for WASM bindings
- **Published v0.1.0 to npm** (@d-o-hub/chaotic-semantic_memory)
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

[unreleased]: https://github.com/d-o-hub/chaotic_semantic_memory/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.1.1
[0.1.0]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.1.0
