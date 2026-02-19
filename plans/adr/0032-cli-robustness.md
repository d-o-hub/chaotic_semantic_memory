# [ADR-0032] CLI Robustness: JSON Escaping, Exit Codes, and Error Output

## Status
Proposed

## Context and Problem Statement
The CLI has three correctness issues that affect production use:

1. **JSON output is not safely escaped**: `print_success()` and command output functions use `format!`/`println!` with raw strings in JSON templates (src/cli/commands/mod.rs:25,40; inject.rs:52-55; associate.rs:50-53). If a concept ID contains `"` or `\n`, the output is invalid JSON.

2. **Exit codes collapse to 255**: CLI command functions return `anyhow::Result`, which `From<anyhow::Error> for CliError` maps to `CliError::Other` (src/cli/error.rs:31-34). This means most failures (validation, DB, IO) exit with code 255 instead of the appropriate typed code (1-7).

3. **Error output ignores `--output-format`**: `main()` always formats errors as table (src/bin/csm.rs:91-93), even when the user requested JSON output.

4. **`--config` flag is declared but unused**: (src/cli/args.rs:15-17) — users will assume it works.

## Decision Drivers
- CLI consumers (scripts, CI) rely on exit codes and valid JSON for automation
- Invalid JSON breaks downstream tooling silently
- Unused flags violate principle of least surprise

## Considered Options
- Option A: Fix all four issues incrementally
- Option B: Rewrite CLI error handling from scratch
- Option C: Remove JSON output format entirely

## Decision Outcome
Chosen option: "Option A — Fix all four issues incrementally", because each fix is independent and low-risk.

### Implementation
1. Replace all `format!`-based JSON output with `serde_json::json!` macro + `serde_json::to_string()`
2. Change CLI command functions to return `cli::Result<()>` (with `CliError`) instead of `anyhow::Result<()>`, using explicit error mapping
3. Pass `output_format` to the error formatter in `main()`
4. Remove unused `--config` flag (or add stub implementation that reads TOML)

### Positive Consequences
- Valid JSON output guaranteed for all concept IDs
- Meaningful exit codes for automated error handling
- Consistent error formatting regardless of output mode
- No unused flags confusing users

### Negative Consequences
- Minor churn in CLI command function signatures
