# Project Learnings

## 2026-05-01 — achieve CLI ↔ Framework API parity (ADR-0066)

### What was fixed
- Implemented 11 missing subcommands to achieve 100% API coverage.
- Persistent operational metrics stored in SQLite (`csm_metrics` table).
- Recursive parser for MongoDB-style metadata filters in CLI.
- Robust `watch` command handling for Unix pipes (EPIPE).
- Non-destructive metrics reset that preserves reservoir state.
- Increased default path traversal depth to 32.

### CI jobs fixed
- **Lint**: Fixed "items after test module" by re-ordering file contents. Fixed "needless borrows" and unused imports.
- **LOC Gate**: Refactored core files (`framework.rs`, `singularity.rs`, `persistence.rs`) into smaller modules and dedicated test files (`*_tests.rs`) to stay under 500 lines.
- **WASM Build**: Fixed missing methods in Persistence stub when feature is disabled.

### Patterns to remember
- **Multi-process CLI**: Ephemeral CLI processes need a shared state (DB/file) for metrics and events to be meaningful across invocations.
- **LOC Management**: Refactor BEFORE adding features if close to the LOC limit. Prefer extracting unit tests to sibling modules (`mod *_tests;`) and splitting large impl blocks.
- **API Parity**: Documented examples in the book should be the gold standard for implementation syntax (e.g. JSON query format).
- **Unix Piping**: Streaming CLI commands must handle `std::io::ErrorKind::BrokenPipe` gracefully.

### Non-fixable issues documented
- **Shared Event Stream**: Cross-process `watch` events currently use a broadcast channel limited to the process instance. Full cross-process notification requires a systemic change (DB triggers or dedicated pub/sub socket) which was deferred.

### Skills used
- **rust-development**: Implementation of CLI subcommands and framework extensions.
- **testing-validation**: New integration test suite `cli_parity.rs`.
- **goap-planning**: Iterative refinement of the implementation plan based on CI/Feedback.
