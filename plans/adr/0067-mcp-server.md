# ADR-0067: MCP Server (`csm mcp serve`)

## Status

Proposed (2026-04-30)

## Context and Problem Statement

The crate's positioning is "production AI memory system". Modern LLM agents (Claude Desktop, Claude Code, Cursor, VS Code Copilot, Continue, OpenAI ChatGPT desktop) consume external context through the **Model Context Protocol** (MCP), an open JSON-RPC 2.0 protocol over stdio or SSE.

Without an MCP surface, every integration requires a custom adapter. Competing memory systems (mem0, Zep, Letta) ship MCP servers as a first-class entry point.

## Decision Drivers

- Zero external dependencies for end users — `csm mcp serve` should "just work"
- Stdio transport (default) for desktop apps; SSE for hosted deployments
- Tool surface must mirror CLI / framework
- LOC budget: ≤ 500 LOC per file
- Must compile with `--no-default-features --features cli,mcp` (opt-in)

## Considered Options

1. **First-party MCP server** behind a `mcp` feature flag using `rmcp` crate (official Rust SDK)
2. Document a third-party adapter recipe and stop there
3. Roll our own JSON-RPC framing + handlers

## Decision Outcome

Chosen: **Option 1** with `rmcp` crate (Anthropic-maintained). Provides:
- Stdio + SSE transports
- Tool / resource / prompt primitives
- Schema generation from Rust types

## Implementation

### Crate dependencies (opt-in)

```toml
[features]
mcp = ["dep:rmcp", "dep:tokio", "cli"]

[dependencies]
rmcp = { version = "0.2", optional = true, features = ["server", "transport-stdio", "transport-sse"] }
```

### New module

`src/mcp/` (5 files, each ≤ 300 LOC):

| File | Responsibility |
|---|---|
| `src/mcp/mod.rs` | Re-exports + `serve()` entry point |
| `src/mcp/server.rs` | RMCP server wiring, transport selection |
| `src/mcp/tools.rs` | 12 MCP tool handlers (one per CLI subcommand) |
| `src/mcp/resources.rs` | Resource provider (concept://id, stats://current) |
| `src/mcp/schema.rs` | JSON Schema definitions for tool inputs |

### Tool surface (12 tools)

| Tool name | Maps to | Description |
|---|---|---|
| `memory_inject` | `inject_concept_with_metadata` | Store a concept |
| `memory_inject_text` | `inject_text` | Store from text (uses TextEncoder) |
| `memory_probe` | `probe` | Top-K similarity search |
| `memory_probe_text` | `probe_text` | Text-query probe |
| `memory_probe_filtered` | `probe_filtered` | Metadata-filtered probe |
| `memory_get` | `get_concept` | Fetch concept by ID |
| `memory_delete` | `delete_concept` | Remove a concept |
| `memory_associate` | `associate` | Create directed association |
| `memory_traverse` | `traverse` | BFS from concept |
| `memory_shortest_path` | `shortest_path` | Path between concepts |
| `memory_stats` | `stats` | DB stats snapshot |
| `memory_export` | `export` | Export to JSON |

### Resource surface

- `concept://{id}` — JSON serialization of one concept
- `stats://current` — live framework stats
- `health://current` — persistence health check

### CLI integration

```
csm mcp serve [--transport stdio|sse] [--bind 127.0.0.1:3030] [--database PATH]
```

Default: stdio transport (Claude Desktop / Cursor compatible).

### Shipping path

- `examples/mcp_claude_desktop.json` — copy-paste config snippet
- `book/src/mcp.md` — installation walkthrough
- `cli-npm/` — bundle MCP feature in npm CLI build

## Pros and Cons

### Pros
- Zero-friction integration with all major LLM clients
- Reuses existing CLI argument parsing (only handler thunks are new)
- Fits the "AI memory" positioning literally

### Cons
- New optional dependency (`rmcp`) — small (~80 KB compiled)
- Schema must stay in sync with framework signatures
- SSE transport requires `tokio::net` — already a tree dep, no real cost

## Acceptance Criteria

- [ ] `cargo build --features mcp` succeeds
- [ ] `csm mcp serve` responds to MCP `initialize` request
- [ ] All 12 tools listable via `tools/list`
- [ ] All 3 resources listable via `resources/list`
- [ ] Smoke test against Claude Desktop config
- [ ] Documentation page in book
- [ ] Each `src/mcp/*.rs` file ≤ 300 LOC
