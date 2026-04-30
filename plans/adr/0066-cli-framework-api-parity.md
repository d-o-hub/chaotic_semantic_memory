# ADR-0066: CLI ↔ Framework API Parity

## Status

Proposed (2026-04-30)

## Context and Problem Statement

`Framework` (src/framework.rs) exposes 22 public async methods. Only 9 are surfaced through the CLI (`inject`, `probe`, `query`, `associate`, `export`, `import`, `index_jsonl`, `index_dir`, `completions`).

Skill-memory shell scripts, dogfooding workflows, and external CLI users cannot exercise:
- `delete_concept`, `get_concept`, `get_associations`
- `update_concept_vector`, `update_concept_metadata`
- `disassociate`, `clear_associations`
- `traverse` (BFS), `shortest_path`
- `probe_filtered` (metadata-filtered search)
- `stats`, `metrics_snapshot`, `persistence_health_check`
- `subscribe` (live event tail)

This forces every non-trivial workflow (a memory cleanup, a graph hop, a metric pull) to drop into Rust or WASM. Comparable systems (mem0, Letta, Zep) expose 100% of their core API at the CLI/REST layer.

## Decision Drivers

- Skill-memory dogfooding requires shell-only workflows
- External adoption blocked by missing operations
- LOC budget: each command file ≤ 250 LOC, `cli/commands/mod.rs` ≤ 200 LOC
- Must preserve existing flags (`--database`, `--git-local`, `--output-format`, `-v`)
- WASM-side stays untouched (separate ADR if needed)

## Considered Options

1. **Add 11 missing subcommands** (one file each under `src/cli/commands/`)
2. Add a single `csm exec <op> [args]` reflection-style command
3. Defer until a user requests a specific operation

## Decision Outcome

Chosen: **Option 1** — add 11 dedicated subcommands. Reflection commands are awkward for shell completions, help text, and JSON output contracts.

## Implementation

### New files (each ≤ 250 LOC)

| File | Subcommand | Framework call |
|---|---|---|
| `src/cli/commands/delete.rs` | `csm delete <id> [--force]` | `delete_concept(id)` |
| `src/cli/commands/get.rs` | `csm get <id>` | `get_concept(id)` |
| `src/cli/commands/update.rs` | `csm update <id> [--vector-from-text TEXT] [--metadata JSON]` | `update_concept_*` |
| `src/cli/commands/disassociate.rs` | `csm disassociate <from> [<to>]` (no `<to>` = clear all) | `disassociate` / `clear_associations` |
| `src/cli/commands/associations.rs` | `csm associations <id> [--reverse]` | `get_associations` / `incoming_associations` |
| `src/cli/commands/traverse.rs` | `csm traverse <start> [--depth N] [--min-strength F]` | `traverse` (BFS) |
| `src/cli/commands/path.rs` | `csm path <from> <to> [--weighted]` | `shortest_path` |
| `src/cli/commands/probe_filtered.rs` | `csm probe-filtered <id> -k N --filter JSON` | `probe_filtered` |
| `src/cli/commands/stats.rs` | `csm stats` | `stats()` |
| `src/cli/commands/metrics.rs` | `csm metrics [--reset]` | `metrics_snapshot()` |
| `src/cli/commands/watch.rs` | `csm watch [--filter EventKind]` | `subscribe()` (channel → stdout JSONL) |

### Wiring

- Add 11 enum variants to `Commands` in `src/cli/mod.rs` (or `src/cli/args.rs`).
- Add 11 dispatch arms to `src/bin/csm.rs` match block (lines 120-150).
- Re-export 11 `run_*` functions from `src/cli/mod.rs`.

### Output contract

- All commands honor `--output-format json|text` (default `text`).
- JSON output uses snake_case fields, includes `status: ok|error`.
- `csm watch` streams JSONL (one event per line, flushed per write).

### Tests

- New integration test file `tests/cli_parity.rs` exercising each subcommand.
- Property test: every Framework public method has a CLI counterpart (compile-time check via macro is overkill — use a doc-string lint).

## Pros and Cons

### Pros
- Restores 100% CLI/Framework parity
- Unblocks shell-based dogfooding
- Each command is a small, isolated file (LOC budget honored)

### Cons
- +11 source files, ~1500 LOC
- 11 new clap subcommand definitions to keep in sync with framework signatures
- Shell completions need regeneration

## Acceptance Criteria

- [ ] All 11 commands implemented and tested
- [ ] `csm --help` lists all 20 subcommands
- [ ] `tests/cli_parity.rs` passes
- [ ] `cargo clippy --all-features -- -D warnings` green
- [ ] All command files ≤ 250 LOC
- [ ] Updated `book/src/cli-reference.md`
- [ ] Updated shell completion regeneration script
