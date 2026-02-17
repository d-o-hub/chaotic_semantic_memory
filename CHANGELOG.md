# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Security
- Added `SECURITY.md` with vulnerability reporting process
- Added `CODE_OF_CONDUCT.md` (Contributor Covenant 2.1)
- Updated CI workflow with security permissions and concurrency controls

### Changed
- Fixed `.github/dependabot.yml` to properly monitor Cargo dependencies
- Updated `actions/cache` from v3 to v4 in CI workflow
- Pinned CI runner to `ubuntu-24.04` for reproducibility
- Added artifact retention policy (30 days)

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
