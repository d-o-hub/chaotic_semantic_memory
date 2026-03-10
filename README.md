# chaotic_semantic_memory

[![CI](https://github.com/d-o-hub/chaotic_semantic_memory/actions/workflows/ci.yml/badge.svg)](https://github.com/d-o-hub/chaotic_semantic_memory/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/chaotic_semantic_memory.svg)](https://crates.io/crates/chaotic_semantic_memory)
[![docs.rs](https://img.shields.io/docsrs/chaotic_semantic_memory)](https://docs.rs/chaotic_semantic_memory)
[![npm](https://img.shields.io/npm/v/@d-o-hub/chaotic_semantic_memory)](https://www.npmjs.com/package/@d-o-hub/chaotic_semantic_memory)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

`chaotic_semantic_memory` is a Rust crate for AI memory systems built on:
- 10240-bit hyperdimensional vectors with SIMD acceleration
- chaotic echo-state reservoirs for temporal processing
- libSQL persistence (local SQLite or remote Turso)

It targets both native and `wasm32` builds with explicit threading guards.

## Quick Links

| Resource | Link |
|----------|------|
| Documentation | [docs.rs/chaotic_semantic_memory](https://docs.rs/chaotic_semantic_memory) |
| Crates.io | [crates.io/crates/chaotic_semantic_memory](https://crates.io/crates/chaotic_semantic_memory) |
| Issues | [GitHub Issues](https://github.com/d-o-hub/chaotic_semantic_memory/issues) |
| Changelog | [CHANGELOG.md](CHANGELOG.md) |

## Features

- **Hyperdimensional Computing**: 10240-bit binary hypervectors with SIMD-accelerated operations
- **Chaotic Reservoirs**: Configurable echo-state networks with spectral radius controls `[0.9, 1.1]`
- **Semantic Memory**: Concept graphs with weighted associations and similarity search
- **Persistence**: libSQL for local SQLite or remote Turso database
- **WASM Support**: Browser-compatible with memory-based import/export
- **CLI**: Full-featured command-line interface with shell completions
- **Production-Ready**: Structured logging, metrics, input validation, memory guardrails

## Installation

```bash
cargo add chaotic_semantic_memory
