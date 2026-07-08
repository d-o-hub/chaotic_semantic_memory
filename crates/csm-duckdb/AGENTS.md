# AGENTS.md - Chaotic Semantic Memory DuckDB Companion

## Mission

Provide high-performance OLAP and analytics capabilities for the `chaotic_semantic_memory` ecosystem using DuckDB.

## Guidelines

- **One-way Dependency**: This crate depends on `chaotic_semantic_memory`, but `chaotic_semantic_memory` must NEVER depend on this crate.
- **WASM Isolation**: This crate is NOT intended for `wasm32` targets. Ensure native-only dependencies are scoped appropriately.
- **LOC Limits**: Follow the 500 lines per file limit for all Rust source files.
- **SQL Best Practices**: Use parameterized queries to prevent SQL injection when interacting with DuckDB.
