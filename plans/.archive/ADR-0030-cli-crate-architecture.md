# ADR-0030: CLI Crate Architecture

## Status
Proposed

## Context and Problem Statement

The `chaotic_semantic_memory` crate needs a command-line interface for:
- Injecting vectors with metadata
- Probing for similar memories  
- Creating associations between memories
- Exporting/importing memory dumps
- Shell integration (completions, scripting)

We need to decide on argument parsing, error handling, crate structure, and output formatting following 2026 Rust best practices.

## Decision Drivers

- Scriptability (exit codes, machine-parseable output)
- User experience (colored output, shell completions)
- Maintainability (subcommand isolation, clear error types)
- Configuration flexibility (layering: CLI > env > file)
- Minimal dependencies in library crate

## Considered Options

1. **clap derive** - Declarative `#[derive(Parser)]` with subcommands
2. **clap builder** - Runtime API construction
3. **argh** - Lightweight derive-based parser
4. **xshell** - Script-oriented CLI

## Decision Outcome

Chosen option: **clap 4.x derive**, because it provides:
- Compile-time validation of arguments
- Built-in help generation and shell completions
- Type-safe subcommand dispatch
- Industry standard with active maintenance

### Positive Consequences
- Automatic `--help` and shell completions via `clap_complete`
- Type-safe argument parsing with no runtime panics
- Clear separation: library errors (`thiserror`) vs app errors (`anyhow`)

### Negative Consequences
- Additional dependency (clap is ~200KB compiled)
- Slight compile-time overhead from derive macros

## Crate Structure

```
src/
├── bin/
│   └── csm-cli.rs          # Binary entry point (< 100 LOC)
├── cli/
│   ├── mod.rs              # CLI exports, run() entry
│   ├── args.rs             # CliArgs, subcommand enum
│   ├── output.rs           # Colored output, NO_COLOR support
│   ├── config.rs           # Config layering (CLI > env > file)
│   └── commands/
│       ├── mod.rs          # Command trait and dispatch
│       ├── inject.rs       # `csm inject` implementation
│       ├── probe.rs        # `csm probe` implementation
│       ├── associate.rs    # `csm associate` implementation
│       ├── export.rs       # `csm export` implementation
│       └── import.rs       # `csm import` implementation
└── lib.rs                  # Library crate (unchanged)
```

### Key Decisions

1. **Error Handling Pattern**
   - Library: `thiserror` for typed errors in `src/error.rs`
   - CLI: `anyhow` for application error context in binary
   - Exit codes: `0`=success, `1`=error, `2`=usage error, `4`=IO error

2. **Configuration Layering** (priority high to low)
   ```
   CLI args > Environment vars (CSM_*) > Config file (~/.config/csm/config.toml)
   ```

3. **Output Formatting**
   - Respect `NO_COLOR` environment variable
   - `--json` flag for machine-parseable output
   - Human-readable tables by default

## Dependencies to Add

```toml
[dependencies]
clap = { version = "4", features = ["derive", "env"] }
clap_complete = "4"
anyhow = "1.0"
colored = "2.1"  # Or termcolor for NO_COLOR support
```

## Example CLI Usage

```bash
# Inject a memory
csm inject --content "meeting notes" --metadata '{"project":"alpha"}'

# Probe for similar memories
csm probe --content "meeting" --limit 5 --json

# Create association
csm associate --source-id abc123 --target-id def456 --strength 0.8

# Export memories
csm export --output memories.bincode --format bincode

# Generate shell completions
csm completions bash > /etc/bash_completion.d/csm

# With config file
csm --config ~/.config/csm/config.toml probe --content "test"
```

## Exit Codes

| Code | Meaning | Use Case |
|------|---------|----------|
| 0 | Success | Command completed |
| 1 | Error | Runtime error (connection, parse) |
| 2 | Usage | Invalid arguments |
| 4 | IO | File/network errors |

## References

- clap 4.x documentation: https://docs.rs/clap/4
- `clap_complete` for shell completions
- `NO_COLOR` spec: https://no-color.org
