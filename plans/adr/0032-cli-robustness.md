# ADR-0032: CLI Robustness

## Status

Accepted (backfilled 2026-05-01) - Wave 10 Complete

## Context

CLI error handling issues:
- JSON output uses format! (incorrect escaping)
- Exit codes collapse to 255 (not mapped)
- Error output ignores --output-format flag
- Unused --config flag

## Decision

Implement **CLI robustness improvements**.

**Deliverables:**
- JSON output: serde_json::json! macro (correct escaping)
- Exit codes: 1-7 mapped to error types (not 255)
- Error format: respects --output-format flag
- Remove unused --config flag

## Consequences

### Positive
- Valid JSON for all concept IDs
- Meaningful exit codes for scripting
- Consistent error formatting
- Cleaner CLI interface

### Negative
- API change for CLI exit codes
- Requires script updates
- Serde overhead for JSON

## Implementation

- Module: src/cli/commands/*.rs, src/cli/error.rs
- Exit codes: CliError enum -> ExitCode mapping
- JSON: serde_json::json! in all output paths

## Sources

- ACTIONS.md lines 1238-1288 (Phase 21 CLI actions)
- ADR_REGISTRY.md: CLI Robustness details
- src/cli/error.rs: CliError -> ExitCode